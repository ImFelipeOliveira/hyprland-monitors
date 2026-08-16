//! Config-store adapter for the Lua provider: rewrites `~/.config/hypr/monitors.lua`.
//!
//! The file is edited line-by-line, never through a Lua parser: lines that are
//! single-line `hl.monitor({...})` calls with a non-empty `output` we manage get
//! replaced; everything else (comments, `hl.env`, the `output = ""` fallback rule,
//! multi-line calls we don't understand) passes through untouched.

use super::managed_lines::{self, Line};
use crate::application::ports::ConfigStore;
use crate::domain::monitor::MonitorState;
use std::path::{Path, PathBuf};

pub struct FileConfigStore {
    pub path: PathBuf,
}

impl FileConfigStore {
    pub fn default_path() -> Result<PathBuf, String> {
        let home = std::env::var_os("HOME").ok_or("HOME environment variable is not set")?;
        Ok(Path::new(&home).join(".config/hypr/monitors.lua"))
    }
}

impl ConfigStore for FileConfigStore {
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
            if trimmed.starts_with("hl.monitor(")
                && trimmed.contains(')')
                && let Some(output) = extract_string_field(trimmed, "output")
            {
                return Line::Entry {
                    key: output,
                    mode: extract_string_field(trimmed, "mode"),
                    raw: line.to_string(),
                };
            }
            Line::Passthrough(line.to_string())
        })
        .collect()
}

/// Extract `field = "value"` from a single-line table constructor.
fn extract_string_field(line: &str, field: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(rel) = line[search_from..].find(field) {
        let idx = search_from + rel;
        // Must be a standalone identifier (not a suffix of another word).
        let before_ok = idx == 0
            || !line[..idx]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
        let rest = &line[idx + field.len()..];
        let rest_trim = rest.trim_start();
        if before_ok && rest_trim.starts_with('=') {
            let after_eq = rest_trim[1..].trim_start();
            if let Some(stripped) = after_eq.strip_prefix('"') {
                let end = stripped.find('"')?;
                return Some(stripped[..end].to_string());
            }
            return None; // field exists but is not a string literal
        }
        search_from = idx + field.len();
    }
    None
}

/// Render one managed monitor entry in the Lua format. If the user never touched
/// the mode this session, keep whatever the file had (typically "preferred") so
/// we don't pin a mode unnecessarily.
fn render_entry(m: &MonitorState, existing_mode: Option<&str>) -> String {
    if m.enabled && !m.mode_touched {
        return m.lua_entry_with_mode(existing_mode.unwrap_or("preferred"));
    }
    m.to_lua_entry()
}

#[cfg(test)]
pub fn serialize(lines: &[Line], entries: &[MonitorState]) -> String {
    managed_lines::rewrite(lines, entries, render_entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::monitor::Mode;

    const OMARCHY_FILE: &str = r#"local omarchy_gdk_scale = 1

hl.env("GDK_SCALE", tostring(omarchy_gdk_scale))

hl.monitor({ output = "HDMI-A-1", mode = "1920x1080@144", position = "0x0", scale = 1 })

-- Seu monitor primário (a tela do seu Lenovo LOQ costuma ser "eDP-1")
-- Ao cravar a scale em 1, você impede que a interface fique desproporcional
hl.monitor({ output = "eDP-1", mode = "preferred", position = "0x1080", scale = 1 })

-- A regra genérica que você tinha fica apenas no final como precaução
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1 })
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
        }
    }

    #[test]
    fn parse_classifies_lines() {
        let lines = parse(OMARCHY_FILE);
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
                .any(|l| matches!(l, Line::Passthrough(s) if s.contains("hl.env")))
        );
    }

    #[test]
    fn roundtrip_preserves_env_comments_and_fallback() {
        let lines = parse(OMARCHY_FILE);
        let entries = vec![
            state("HDMI-A-1", (1920, 0), true),
            state("eDP-1", (0, 0), false),
        ];
        let out = serialize(&lines, &entries);

        assert!(out.contains("hl.env(\"GDK_SCALE\""));
        assert!(out.contains("-- Seu monitor primário"));
        // Fallback rule survives and stays after the managed entries.
        let fallback_pos = out.find("output = \"\"").unwrap();
        let edp_pos = out.find("output = \"eDP-1\"").unwrap();
        assert!(edp_pos < fallback_pos);
        // Touched mode is pinned; untouched keeps the file's "preferred".
        assert!(out.contains(
            "hl.monitor({ output = \"HDMI-A-1\", mode = \"1920x1080@144\", position = \"1920x0\", scale = 1 })"
        ));
        assert!(out.contains(
            "hl.monitor({ output = \"eDP-1\", mode = \"preferred\", position = \"0x0\", scale = 1 })"
        ));
    }

    #[test]
    fn disabled_monitor_is_persisted_as_disabled() {
        let lines = parse(OMARCHY_FILE);
        let mut hdmi = state("HDMI-A-1", (0, 0), true);
        hdmi.enabled = false;
        let out = serialize(&lines, &[hdmi, state("eDP-1", (0, 0), false)]);
        assert!(out.contains("hl.monitor({ output = \"HDMI-A-1\", disabled = true })"));
    }

    #[test]
    fn new_monitor_is_inserted_before_fallback() {
        let content = "-- só fallback\nhl.monitor({ output = \"\", mode = \"preferred\", position = \"auto\", scale = 1 })\n";
        let out = serialize(&parse(content), &[state("DP-3", (0, 0), false)]);
        let dp = out.find("output = \"DP-3\"").unwrap();
        let fb = out.find("output = \"\"").unwrap();
        assert!(dp < fb);
        assert!(out.contains("mode = \"preferred\""));
    }

    #[test]
    fn empty_file_gets_entries_appended() {
        let out = serialize(&parse(""), &[state("eDP-1", (0, 0), true)]);
        assert!(
            out.trim_start()
                .starts_with("hl.monitor({ output = \"eDP-1\"")
        );
    }

    #[test]
    fn extract_field_ignores_lookalike_names() {
        let line = r#"hl.monitor({ xoutput = "no", output = "DP-1", mode = "preferred" })"#;
        assert_eq!(
            extract_string_field(line, "output").as_deref(),
            Some("DP-1")
        );
    }

    #[test]
    fn file_store_persists_with_backup() {
        let dir =
            std::env::temp_dir().join(format!("hyprland-monitors-test-lua-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("monitors.lua");
        std::fs::write(&path, OMARCHY_FILE).unwrap();

        let store = FileConfigStore { path: path.clone() };
        store
            .persist(&[
                state("HDMI-A-1", (1920, 0), true),
                state("eDP-1", (0, 0), false),
            ])
            .unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("position = \"1920x0\""));
        // Backup holds the pre-write content.
        let bak = std::fs::read_to_string(dir.join("monitors.lua.bak")).unwrap();
        assert_eq!(bak, OMARCHY_FILE);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
