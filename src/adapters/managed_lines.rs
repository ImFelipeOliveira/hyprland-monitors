//! Shared core for both config stores (Lua and classic .conf): a config file is
//! treated as a list of lines where some are "managed" monitor entries (keyed by
//! output name) and the rest pass through verbatim. Rewriting replaces the managed
//! block in place; writes are backed up and atomic.

use crate::domain::monitor::MonitorState;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Passthrough(String),
    /// A monitor entry. `key` is the output name; an empty key is the generic
    /// fallback rule, which is preserved but never treated as managed.
    Entry {
        key: String,
        mode: Option<String>,
        raw: String,
    },
}

/// Rewrite the file: managed entries are replaced as a block at the position of
/// the first pre-existing managed entry (or before the fallback rule, or appended),
/// in the order given by `entries`. All other lines are preserved verbatim.
/// `render` receives each monitor plus the mode string the file previously had
/// for it (so untouched modes can keep e.g. "preferred").
pub fn rewrite(
    lines: &[Line],
    entries: &[MonitorState],
    render: impl Fn(&MonitorState, Option<&str>) -> String,
) -> String {
    let managed: HashSet<&str> = entries.iter().map(|m| m.name.as_str()).collect();
    let existing_modes: HashMap<String, String> = lines
        .iter()
        .filter_map(|l| match l {
            Line::Entry {
                key,
                mode: Some(mode),
                ..
            } if !key.is_empty() => Some((key.clone(), mode.clone())),
            _ => None,
        })
        .collect();

    // Drop managed lines, remembering where the block should be inserted.
    let mut kept: Vec<&Line> = Vec::new();
    let mut insert_at: Option<usize> = None;
    for line in lines {
        if let Line::Entry { key, .. } = line {
            if !key.is_empty() && managed.contains(key.as_str()) {
                insert_at.get_or_insert(kept.len());
                continue;
            }
            if key.is_empty() && insert_at.is_none() {
                // No managed entry seen yet: place the block before the fallback rule.
                insert_at = Some(kept.len());
            }
        }
        kept.push(line);
    }
    let insert_at = insert_at.unwrap_or(kept.len());

    let mut out: Vec<String> = Vec::with_capacity(kept.len() + entries.len());
    let render_block = |out: &mut Vec<String>| {
        for m in entries {
            out.push(render(m, existing_modes.get(&m.name).map(String::as_str)));
        }
    };
    for (i, line) in kept.iter().enumerate() {
        if i == insert_at {
            render_block(&mut out);
        }
        out.push(match line {
            Line::Passthrough(s) => s.clone(),
            Line::Entry { raw, .. } => raw.clone(),
        });
    }
    if insert_at >= kept.len() {
        render_block(&mut out);
    }
    let mut result = out.join("\n");
    result.push('\n');
    result
}

/// Current file content; an empty string when the file doesn't exist yet.
pub fn load(path: &Path) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("failed to read {}: {e}", path.display())),
    }
}

/// Backup to `<filename>.bak`, then write atomically (temp file + rename in the same dir).
pub fn save(path: &Path, content: &str) -> Result<(), String> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("config path has no file name")?;
    if path.exists() {
        let bak = path.with_file_name(format!("{file_name}.bak"));
        std::fs::copy(path, &bak)
            .map_err(|e| format!("failed to create backup {}: {e}", bak.display()))?;
    }
    let dir = path.parent().ok_or("config path has no parent directory")?;
    let tmp = dir.join(format!(".{file_name}.tmp"));
    std::fs::write(&tmp, content).map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("failed to replace {}: {e}", path.display())
    })
}
