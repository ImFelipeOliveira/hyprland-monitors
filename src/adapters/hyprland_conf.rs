//! Config-store adapter for the classic .conf provider: rewrites
//! `~/.config/hypr/monitors.conf`.
//!
//! Managed lines are single `monitor = <name>, <mode>, <pos>, <scale>` entries
//! (a `monitor = , ...` line with an empty name is the generic fallback rule,
//! preserved but never managed). Everything else — comments, `monitorv2`
//! blocks, unrelated keywords — passes through untouched. The user is expected
//! to `source` this file from hyprland.conf (documented in the README).

use super::managed_lines::{self, Line};
use crate::application::ports::ConfigStore;
use crate::domain::monitor::MonitorState;
use std::path::{Path, PathBuf};

pub struct ConfConfigStore {
    pub path: PathBuf,
}

impl ConfConfigStore {
    pub fn default_path() -> Result<PathBuf, String> {
        let home = std::env::var_os("HOME").ok_or("HOME environment variable is not set")?;
        Ok(Path::new(&home).join(".config/hypr/monitors.conf"))
    }
}

impl ConfigStore for ConfConfigStore {
    fn persist(&self, monitors: &[MonitorState]) -> Result<(), String> {
        let content = managed_lines::load(&self.path)?;
        let lines = parse(&content);
        managed_lines::save(
            &self.path,
            &managed_lines::rewrite(&lines, monitors, render_entry),
        )
    }
}

pub fn parse(content: &str) -> Vec<Line> {
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            // `monitor = ...` / `monitor=...`, but NOT `monitorv2 { ... }`.
            if let Some(rest) = trimmed.strip_prefix("monitor")
                && let Some(value) = rest.trim_start().strip_prefix('=')
            {
                let fields: Vec<&str> = value.split(',').map(str::trim).collect();
                let key = fields.first().copied().unwrap_or("").to_string();
                let mode = fields
                    .get(1)
                    .filter(|m| !m.is_empty() && !m.eq_ignore_ascii_case("disable"))
                    .map(|m| m.to_string());
                return Line::Entry {
                    key,
                    mode,
                    raw: line.to_string(),
                };
            }
            Line::Passthrough(line.to_string())
        })
        .collect()
}

/// Render one managed entry in classic format, keeping the file's existing mode
/// string (typically "preferred") when the user never touched the mode.
fn render_entry(m: &MonitorState, existing_mode: Option<&str>) -> String {
    if m.enabled && !m.mode_touched {
        return m.conf_line_with_mode(existing_mode.unwrap_or("preferred"));
    }
    m.to_conf_line()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::monitor::Mode;

    const CONF_FILE: &str = r#"# Monitor layout — see https://wiki.hypr.land/Configuring/Monitors/
monitor = HDMI-A-1, 1920x1080@144, 0x0, 1
# laptop panel
monitor = eDP-1, preferred, 0x1080, 1

monitorv2 {
    output = DP-9
}

# generic fallback
monitor = , preferred, auto, 1
"#;

    fn state(name: &str, pos: (i32, i32), touched: bool) -> MonitorState {
        MonitorState {
            name: name.to_string(),
            description: String::new(),
            modes: vec![Mode {
                width: 1920,
                height: 1080,
                refresh: 144.0,
            }],
            mode: Mode {
                width: 1920,
                height: 1080,
                refresh: 144.0,
            },
            mode_touched: touched,
            pos,
            scale: 1.0,
            enabled: true,
            transform: 0,
            vrr: 0,
            mirror_of: None,
        }
    }

    fn serialize(lines: &[Line], entries: &[MonitorState]) -> String {
        managed_lines::rewrite(lines, entries, render_entry)
    }

    #[test]
    fn parse_classifies_lines_and_skips_monitorv2() {
        let lines = parse(CONF_FILE);
        let monitors: Vec<_> = lines
            .iter()
            .filter_map(|l| match l {
                Line::Entry { key, .. } => Some(key.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(monitors, vec!["HDMI-A-1", "eDP-1", ""]);
        assert!(
            lines
                .iter()
                .any(|l| matches!(l, Line::Passthrough(s) if s.contains("monitorv2")))
        );
    }

    #[test]
    fn roundtrip_preserves_comments_blocks_and_fallback() {
        let lines = parse(CONF_FILE);
        let entries = vec![
            state("HDMI-A-1", (1920, 0), true),
            state("eDP-1", (0, 0), false),
        ];
        let out = serialize(&lines, &entries);

        assert!(out.contains("# Monitor layout"));
        assert!(out.contains("monitorv2 {"));
        // Fallback rule survives and stays after the managed entries.
        let fallback_pos = out.find("monitor = , preferred, auto, 1").unwrap();
        let edp_pos = out.find("monitor = eDP-1").unwrap();
        assert!(edp_pos < fallback_pos);
        // Touched mode is pinned; untouched keeps the file's "preferred".
        assert!(out.contains("monitor = HDMI-A-1, 1920x1080@144, 1920x0, 1"));
        assert!(out.contains("monitor = eDP-1, preferred, 0x0, 1"));
    }

    #[test]
    fn disabled_monitor_is_persisted_as_disable() {
        let lines = parse(CONF_FILE);
        let mut hdmi = state("HDMI-A-1", (0, 0), true);
        hdmi.enabled = false;
        let out = serialize(&lines, &[hdmi, state("eDP-1", (0, 0), false)]);
        assert!(out.contains("monitor = HDMI-A-1, disable"));
        // The old disable-less mode string must not leak back for HDMI.
        assert!(!out.contains("monitor = HDMI-A-1, 1920x1080@144"));
    }

    #[test]
    fn new_monitor_inserted_before_fallback_in_empty_style_file() {
        let content = "# only fallback\nmonitor=,preferred,auto,1\n";
        let out = serialize(&parse(content), &[state("DP-3", (0, 0), false)]);
        let dp = out.find("monitor = DP-3").unwrap();
        let fb = out.find("monitor=,preferred,auto,1").unwrap();
        assert!(dp < fb);
    }

    #[test]
    fn conf_store_persists_with_backup() {
        let dir = std::env::temp_dir().join(format!(
            "hyprland-monitors-test-conf-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("monitors.conf");
        std::fs::write(&path, CONF_FILE).unwrap();

        let store = ConfConfigStore { path: path.clone() };
        store
            .persist(&[
                state("HDMI-A-1", (1920, 0), true),
                state("eDP-1", (0, 0), false),
            ])
            .unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("monitor = HDMI-A-1, 1920x1080@144, 1920x0, 1"));
        let bak = std::fs::read_to_string(dir.join("monitors.conf.bak")).unwrap();
        assert_eq!(bak, CONF_FILE);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
