//! Compositor adapters: talk to the running Hyprland via the `hyprctl` binary
//! (state queries and live changes) and its socket2 event socket (hotplug).
//!
//! Two apply mechanisms exist, selected by `detect_provider()`:
//! - Lua provider (e.g. Omarchy): `hyprctl eval '<lua chunk>'`. The classic
//!   `keyword` command is rejected there ("keyword can't work with non-legacy
//!   parsers. Use eval.").
//! - Classic .conf provider: `hyprctl --batch "keyword monitor ...; ..."`.
//!
//! Either way the whole layout goes out in one request — sequential per-monitor
//! application creates transient overlaps Hyprland rejects.

use crate::application::ports::Compositor;
use crate::domain::monitor::{MonitorState, RawMonitor, parse_monitors_json};
use std::io::BufRead;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigProvider {
    Lua,
    Classic,
}

/// `hyprctl systeminfo` reports `configProvider: lua` on Lua-configured systems;
/// anything else (or the line's absence on older Hyprland) means classic.
pub fn detect_provider() -> ConfigProvider {
    let Ok(out) = Command::new("hyprctl").arg("systeminfo").output() else {
        return ConfigProvider::Classic;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let is_lua = text
        .lines()
        .any(|l| l.trim().starts_with("configProvider:") && l.contains("lua"));
    if is_lua {
        ConfigProvider::Lua
    } else {
        ConfigProvider::Classic
    }
}

/// Adapter for the Lua config provider: one `hl.monitor({...})` chunk via eval.
pub struct EvalCompositor;

impl Compositor for EvalCompositor {
    fn query(&self) -> Result<Vec<RawMonitor>, String> {
        query()
    }
    fn apply_layout(&self, monitors: &[MonitorState]) -> Result<(), String> {
        let chunk: Vec<String> = monitors.iter().map(|m| m.to_live_lua_entry()).collect();
        run_expecting_ok(&["eval", &chunk.join("\n")])
    }
}

/// Adapter for the classic .conf provider: batched `keyword monitor` commands.
pub struct KeywordCompositor;

impl Compositor for KeywordCompositor {
    fn query(&self) -> Result<Vec<RawMonitor>, String> {
        query()
    }
    fn apply_layout(&self, monitors: &[MonitorState]) -> Result<(), String> {
        let batch: Vec<String> = monitors
            .iter()
            .map(|m| format!("keyword monitor {}", m.to_live_keyword_arg()))
            .collect();
        run_expecting_ok(&["--batch", &batch.join(" ; ")])
    }
}

pub fn query() -> Result<Vec<RawMonitor>, String> {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return Err(
            "HYPRLAND_INSTANCE_SIGNATURE is not set — this application must run inside a Hyprland session.".into(),
        );
    }
    let out = Command::new("hyprctl")
        .args(["-j", "monitors", "all"])
        .output()
        .map_err(|e| format!("could not execute hyprctl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "hyprctl failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse_monitors_json(&String::from_utf8_lossy(&out.stdout))
}

/// Run hyprctl and require every non-empty reply line to be "ok".
fn run_expecting_ok(args: &[&str]) -> Result<(), String> {
    let out = Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| format!("could not execute hyprctl: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let reply = stdout.trim();
    let all_ok = !reply.is_empty()
        && reply
            .lines()
            .all(|l| l.trim().is_empty() || l.trim().eq_ignore_ascii_case("ok"));
    if !out.status.success() || !all_ok {
        let detail = if reply.is_empty() {
            String::from_utf8_lossy(&out.stderr).trim().to_string()
        } else {
            reply.to_string()
        };
        return Err(detail);
    }
    Ok(())
}

/// Watch Hyprland's event socket on a background thread; invoke `on_change`
/// whenever a monitor is added or removed.
pub fn spawn_event_listener(on_change: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") else {
            return;
        };
        let Some(signature) = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE") else {
            return;
        };
        let path = std::path::Path::new(&runtime_dir)
            .join("hypr")
            .join(&signature)
            .join(".socket2.sock");
        let Ok(stream) = std::os::unix::net::UnixStream::connect(&path) else {
            return;
        };
        let reader = std::io::BufReader::new(stream);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.starts_with("monitoradded") || line.starts_with("monitorremoved") {
                on_change();
            }
        }
    });
}
