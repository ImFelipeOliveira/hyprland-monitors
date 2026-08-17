//! Monitor domain types: display modes, the editable per-monitor state, and the
//! raw shape reported by the compositor. Pure — no IO, no UI.

use serde::Deserialize;

/// A display mode as advertised by Hyprland (e.g. "1920x1080@144.00Hz").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh: f32,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        let s = s.trim().trim_end_matches("Hz");
        let (res, hz) = s.split_once('@')?;
        let (w, h) = res.split_once('x')?;
        Some(Mode {
            width: w.trim().parse().ok()?,
            height: h.trim().parse().ok()?,
            refresh: hz.trim().parse().ok()?,
        })
    }

    /// "1920x1080@144" — the form accepted by hl.monitor's `mode` field.
    pub fn to_config_string(self) -> String {
        format!(
            "{}x{}@{}",
            self.width,
            self.height,
            format_refresh(self.refresh)
        )
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}x{} @ {} Hz",
            self.width,
            self.height,
            format_refresh(self.refresh)
        )
    }
}

/// Hyprland reports refreshes like 144.00200 or 59.94; keep decimals only when meaningful.
pub fn format_refresh(r: f32) -> String {
    if (r - r.round()).abs() < 0.05 {
        format!("{}", r.round() as u32)
    } else {
        let s = format!("{r:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

pub fn format_scale(s: f32) -> String {
    let out = format!("{s:.4}");
    out.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Raw shape of one entry from `hyprctl -j monitors all`. Unknown fields are ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMonitor {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    #[serde(default)]
    pub transform: u8,
    #[serde(default)]
    pub vrr: bool,
    #[serde(default)]
    pub mirror_of: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub available_modes: Vec<String>,
}

pub fn parse_monitors_json(json: &str) -> Result<Vec<RawMonitor>, String> {
    serde_json::from_str(json).map_err(|e| format!("resposta inesperada do hyprctl: {e}"))
}

/// Editable state of one monitor — used both as the detected snapshot and as the candidate layout.
#[derive(Debug, Clone)]
pub struct MonitorState {
    pub name: String,
    pub description: String,
    pub modes: Vec<Mode>,
    pub mode: Mode,
    /// True once the user explicitly picked a mode this session; controls whether
    /// persistence pins the mode or keeps the file's existing string (e.g. "preferred").
    pub mode_touched: bool,
    pub pos: (i32, i32),
    pub scale: f32,
    pub enabled: bool,
    /// Hyprland transform: 0 normal, 1/2/3 = 90°/180°/270°, 4–7 flipped variants.
    pub transform: u8,
    /// VRR: 0 off, 1 on, 2 fullscreen-only. Note: `hyprctl -j` reports only a
    /// boolean, so mode 2 reads back as on/off after a resync.
    pub vrr: u8,
    /// Output this monitor mirrors, if any.
    pub mirror_of: Option<String>,
}

/// Human-readable labels for the eight Hyprland transform values, index = value.
pub const TRANSFORM_LABELS: [&str; 8] = [
    "Normal",
    "90°",
    "180°",
    "270°",
    "Flipped",
    "Flipped 90°",
    "Flipped 180°",
    "Flipped 270°",
];

/// Labels for VRR modes, index = value.
pub const VRR_LABELS: [&str; 3] = ["Off", "On", "Fullscreen only"];

impl MonitorState {
    pub fn from_raw(raw: &RawMonitor) -> MonitorState {
        let mut modes: Vec<Mode> = raw
            .available_modes
            .iter()
            .filter_map(|m| Mode::parse(m))
            .collect();
        let fallback = Mode {
            width: 1920,
            height: 1080,
            refresh: 60.0,
        };
        if modes.is_empty() {
            let w = if raw.width > 0 {
                raw.width
            } else {
                fallback.width
            };
            let h = if raw.height > 0 {
                raw.height
            } else {
                fallback.height
            };
            let r = if raw.refresh_rate > 0.0 {
                raw.refresh_rate
            } else {
                fallback.refresh
            };
            modes.push(Mode {
                width: w,
                height: h,
                refresh: r,
            });
        }
        let mode = modes
            .iter()
            .copied()
            .find(|m| {
                m.width == raw.width
                    && m.height == raw.height
                    && (m.refresh - raw.refresh_rate).abs() < 0.5
            })
            .unwrap_or(modes[0]);
        MonitorState {
            name: raw.name.clone(),
            description: raw.description.clone(),
            modes,
            mode,
            mode_touched: false,
            pos: (raw.x, raw.y),
            scale: if raw.scale > 0.0 { raw.scale } else { 1.0 },
            enabled: !raw.disabled,
            transform: raw.transform.min(7),
            vrr: raw.vrr as u8,
            mirror_of: match raw.mirror_of.as_str() {
                "" | "none" => None,
                other => Some(other.to_string()),
            },
        }
    }

    /// True for 90°/270° (and their flipped variants): width and height swap.
    pub fn is_rotated(&self) -> bool {
        self.transform % 2 == 1
    }

    /// Size in logical pixels (resolution divided by scale, swapped when rotated)
    /// — what positions are expressed in.
    pub fn logical_size(&self) -> (i32, i32) {
        let w = ((self.mode.width as f32 / self.scale).round() as i32).max(1);
        let h = ((self.mode.height as f32 / self.scale).round() as i32).max(1);
        if self.is_rotated() { (h, w) } else { (w, h) }
    }

    /// One `hl.monitor({...})` Lua call with an explicit mode string. This single
    /// format is accepted by both `hyprctl eval` (live apply) and monitors.lua
    /// (persistence) — Hyprland configured in Lua rejects the classic `keyword`
    /// command ("keyword can't work with non-legacy parsers").
    pub fn lua_entry_with_mode(&self, mode: &str) -> String {
        if !self.enabled {
            return format!(
                "hl.monitor({{ output = \"{}\", disabled = true }})",
                self.name
            );
        }
        let mut extras = String::new();
        if self.transform != 0 {
            extras.push_str(&format!(", transform = {}", self.transform));
        }
        if self.vrr != 0 {
            extras.push_str(&format!(", vrr = {}", self.vrr));
        }
        if let Some(src) = &self.mirror_of {
            extras.push_str(&format!(", mirror = \"{src}\""));
        }
        format!(
            "hl.monitor({{ output = \"{}\", mode = \"{}\", position = \"{}x{}\", scale = {}{} }})",
            self.name,
            mode,
            self.pos.0,
            self.pos.1,
            format_scale(self.scale),
            extras
        )
    }

    /// Lua entry pinning the currently selected mode.
    pub fn to_lua_entry(&self) -> String {
        self.lua_entry_with_mode(&self.mode.to_config_string())
    }

    /// Argument for `hyprctl keyword monitor <arg>` (classic .conf provider).
    pub fn to_keyword_arg(&self) -> String {
        if !self.enabled {
            return format!("{},disable", self.name);
        }
        format!(
            "{},{},{}x{},{}{}",
            self.name,
            self.mode.to_config_string(),
            self.pos.0,
            self.pos.1,
            format_scale(self.scale),
            self.classic_extras(",")
        )
    }

    /// `, transform, N`-style suffix shared by the keyword arg and the conf line;
    /// `sep` is "," for keywords and ", " for the conf file.
    fn classic_extras(&self, sep: &str) -> String {
        let mut extras = String::new();
        if self.transform != 0 {
            extras.push_str(&format!("{sep}transform{sep}{}", self.transform));
        }
        if self.vrr != 0 {
            extras.push_str(&format!("{sep}vrr{sep}{}", self.vrr));
        }
        if let Some(src) = &self.mirror_of {
            extras.push_str(&format!("{sep}mirror{sep}{src}"));
        }
        extras
    }

    /// One `monitor = ...` line for monitors.conf (classic provider), with an
    /// explicit mode string.
    pub fn conf_line_with_mode(&self, mode: &str) -> String {
        if !self.enabled {
            return format!("monitor = {}, disable", self.name);
        }
        format!(
            "monitor = {}, {}, {}x{}, {}{}",
            self.name,
            mode,
            self.pos.0,
            self.pos.1,
            format_scale(self.scale),
            self.classic_extras(", ")
        )
    }

    /// monitors.conf line pinning the currently selected mode.
    pub fn to_conf_line(&self) -> String {
        self.conf_line_with_mode(&self.mode.to_config_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/monitors_all.json");

    #[test]
    fn parses_real_hyprctl_output() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        assert_eq!(raws.len(), 3);
        let edp = raws.iter().find(|r| r.name == "eDP-1").unwrap();
        assert_eq!(edp.description, "BOE 0x0C29 0x00000067");
        assert_eq!((edp.width, edp.height), (1920, 1080));
        assert_eq!((edp.x, edp.y), (0, 1080));
        assert!(!edp.disabled);
        assert_eq!(edp.available_modes.len(), 2);
    }

    #[test]
    fn disabled_monitor_is_enumerated() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        let dp = raws.iter().find(|r| r.name == "DP-1").unwrap();
        assert!(dp.disabled);
        let state = MonitorState::from_raw(dp);
        assert!(!state.enabled);
        assert!(!state.modes.is_empty());
        assert_eq!(
            state.to_lua_entry(),
            "hl.monitor({ output = \"DP-1\", disabled = true })"
        );
    }

    #[test]
    fn state_from_raw_picks_current_mode() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        let edp = raws.iter().find(|r| r.name == "eDP-1").unwrap();
        let state = MonitorState::from_raw(edp);
        assert_eq!(state.mode.width, 1920);
        assert!((state.mode.refresh - 144.002).abs() < 0.01);
        assert_eq!(
            state.to_lua_entry(),
            "hl.monitor({ output = \"eDP-1\", mode = \"1920x1080@144\", position = \"0x1080\", scale = 1 })"
        );
    }

    #[test]
    fn mode_parse_and_format() {
        let m = Mode::parse("2560x1440@59.94Hz").unwrap();
        assert_eq!((m.width, m.height), (2560, 1440));
        assert_eq!(m.to_config_string(), "2560x1440@59.94");
        assert_eq!(
            Mode::parse("1920x1080@144.00200Hz")
                .unwrap()
                .to_config_string(),
            "1920x1080@144"
        );
        assert!(Mode::parse("garbage").is_none());
    }

    #[test]
    fn logical_size_uses_scale() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        let hdmi = raws.iter().find(|r| r.name == "HDMI-A-1").unwrap();
        let mut state = MonitorState::from_raw(hdmi);
        state.scale = 2.0;
        assert_eq!(state.logical_size(), (960, 540));
    }

    #[test]
    fn keyword_and_conf_renderings() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        let edp = MonitorState::from_raw(raws.iter().find(|r| r.name == "eDP-1").unwrap());
        assert_eq!(edp.to_keyword_arg(), "eDP-1,1920x1080@144,0x1080,1");
        assert_eq!(
            edp.to_conf_line(),
            "monitor = eDP-1, 1920x1080@144, 0x1080, 1"
        );
        let dp = MonitorState::from_raw(raws.iter().find(|r| r.name == "DP-1").unwrap());
        assert_eq!(dp.to_keyword_arg(), "DP-1,disable");
        assert_eq!(dp.to_conf_line(), "monitor = DP-1, disable");
    }

    #[test]
    fn rotation_swaps_logical_size() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        let mut edp = MonitorState::from_raw(raws.iter().find(|r| r.name == "eDP-1").unwrap());
        assert_eq!(edp.logical_size(), (1920, 1080));
        edp.transform = 1; // 90°
        assert_eq!(edp.logical_size(), (1080, 1920));
        edp.transform = 2; // 180° — no swap
        assert_eq!(edp.logical_size(), (1920, 1080));
        edp.transform = 5; // flipped 90° — swap
        assert_eq!(edp.logical_size(), (1080, 1920));
    }

    #[test]
    fn advanced_fields_render_in_all_formats() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        let mut m = MonitorState::from_raw(raws.iter().find(|r| r.name == "eDP-1").unwrap());
        m.transform = 3;
        m.vrr = 1;
        m.mirror_of = Some("HDMI-A-1".to_string());
        assert_eq!(
            m.to_lua_entry(),
            "hl.monitor({ output = \"eDP-1\", mode = \"1920x1080@144\", position = \"0x1080\", scale = 1, transform = 3, vrr = 1, mirror = \"HDMI-A-1\" })"
        );
        assert_eq!(
            m.to_keyword_arg(),
            "eDP-1,1920x1080@144,0x1080,1,transform,3,vrr,1,mirror,HDMI-A-1"
        );
        assert_eq!(
            m.to_conf_line(),
            "monitor = eDP-1, 1920x1080@144, 0x1080, 1, transform, 3, vrr, 1, mirror, HDMI-A-1"
        );
    }

    #[test]
    fn default_advanced_fields_keep_v01_output() {
        let raws = parse_monitors_json(FIXTURE).unwrap();
        let m = MonitorState::from_raw(raws.iter().find(|r| r.name == "eDP-1").unwrap());
        assert_eq!(m.transform, 0);
        assert_eq!(m.vrr, 0);
        assert_eq!(m.mirror_of, None);
        // Byte-identical to the v0.1.0 renderings.
        assert_eq!(
            m.to_lua_entry(),
            "hl.monitor({ output = \"eDP-1\", mode = \"1920x1080@144\", position = \"0x1080\", scale = 1 })"
        );
        assert_eq!(m.to_keyword_arg(), "eDP-1,1920x1080@144,0x1080,1");
    }

    #[test]
    fn scale_formatting() {
        assert_eq!(format_scale(1.0), "1");
        assert_eq!(format_scale(1.25), "1.25");
        assert_eq!(format_scale(1.5), "1.5");
    }
}
