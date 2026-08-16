# Proposal: hyprland-monitor-ui

## Why

Managing monitor layout on Omarchy today requires hand-editing `~/.config/hypr/monitors.lua` (`hl.monitor({...})` calls) and reloading Hyprland — error-prone, slow, and unfriendly when docking/undocking a laptop. Existing GUI tools (nwg-displays, wdisplays) write the classic `monitors.conf` format and cannot target Omarchy's Lua-based config.

## What Changes

- New desktop application (Rust + egui) that shows connected monitors on a drag-and-drop canvas and lets the user arrange them spatially (above, below, left, right, or free positioning with edge snapping).
- Per-monitor controls: enable/disable, resolution + refresh rate, and scale.
- Live preview: changes are applied immediately via `hyprctl eval` with `hl.monitor({...})` Lua entries with a confirm/revert countdown, so a bad layout never leaves the user stuck.
- Persistence: confirmed layouts are written to `~/.config/hypr/monitors.lua` in Omarchy's `hl.monitor({...})` format, keeping the config valid for normal Hyprland startup.
- Generic across config providers: the app detects at runtime whether Hyprland runs the Lua or the classic (.conf) config provider, and uses the matching apply command (`eval` vs `keyword --batch`) and persistence target (`monitors.lua` vs `monitors.conf`). Omarchy is one supported case, not a requirement.

## Capabilities

### New Capabilities

- `monitor-detection`: enumerate connected monitors and their current state (name, description, available modes, position, scale, enabled) from Hyprland via `hyprctl -j monitors all`.
- `layout-editor`: drag-and-drop canvas rendering monitors as proportional rectangles, with edge snapping, overlap prevention, and per-monitor settings (mode, scale, enable/disable).
- `live-apply`: apply a candidate layout to the running compositor via `hyprctl eval`, with a timed confirm dialog that automatically reverts to the previous layout if not confirmed.
- `config-persistence`: serialize the confirmed layout to `~/.config/hypr/monitors.lua` as `hl.monitor({...})` entries, preserving non-monitor content in the file (env vars, comments, fallback rule).
- `config-style-detection`: detect which config provider (Lua or classic .conf) the running Hyprland uses, selecting the matching live-apply mechanism and persistence format.

### Modified Capabilities

_None — this is a greenfield project with no existing specs._

## Impact

- New Rust crate (binary) in this repository; dependencies: `eframe`/`egui`, `serde`/`serde_json`.
- Runtime dependency on `hyprctl` (present on any Hyprland/Omarchy system); reads `$HYPRLAND_INSTANCE_SIGNATURE` implicitly through it.
- Writes to `~/.config/hypr/monitors.lua` — the only user file the app touches; a backup is taken before each write.
- No changes to Omarchy itself or to other Hyprland config files.
