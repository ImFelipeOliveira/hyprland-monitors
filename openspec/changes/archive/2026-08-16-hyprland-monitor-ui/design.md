# Design: hyprland-monitor-ui

## Context

- Target environment: Omarchy (Arch + Hyprland) with Lua-based config. Monitor config lives in `~/.config/hypr/monitors.lua` as `hl.monitor({ output, mode, position, scale })` calls, alongside env-var lines (`hl.env`) and comments.
- Hyprland exposes everything needed at runtime: `hyprctl -j monitors all` for state (including disabled monitors and available modes), `hyprctl eval '<lua>'` for live changes (a Lua-configured Hyprland rejects the classic `keyword` command), and a UNIX event socket (`socket2`) for hotplug events.
- Stack decided with the user: **Rust + egui (via eframe)**, drag-and-drop canvas.

## Goals / Non-Goals

**Goals:**
- Single static binary, launchable from Omarchy's app launcher, no runtime deps beyond `hyprctl`.
- Safe-by-default: live preview with auto-revert; backup before persisting.
- Correct round-trip: state read from Hyprland → edited on canvas → applied live → persisted to Lua.

**Non-Goals:**
- Transform/rotation, mirroring, VRR, HDR/color management, per-monitor wallpaper — possible future changes, out of scope for v1.
- Supporting the classic `monitors.conf` format or non-Omarchy setups.
- A daemon/auto-profile system (kanshi-style). This is an on-demand UI.

## Decisions

1. **Crate layout — light ports & adapters** (decided with the user: adapters only where they pay off), with the directory tree mirroring the layers: `domain/` (`monitor.rs`, `geometry.rs`) is pure logic with no IO or UI; `application/` holds `ports.rs` (traits `Compositor` and `ConfigStore`, the only IO boundaries) and `session.rs`, the application service owning the candidate layout and the apply → confirm → keep/revert → persist state machine, depending only on the ports — the whole flow is unit-tested with in-memory fakes; `adapters/` implements the ports (`hyprctl.rs` for query/eval/event socket, `omarchy_lua.rs` for the monitors.lua rewrite — the Lua-format knowledge lives entirely behind `ConfigStore::persist`, invisible to the application layer); `ui/` is a thin egui view split into `mod.rs` (frame loop), `canvas.rs`, `panels.rs`, `dialogs.rs`. Deliberately NOT abstracted: egui itself and the filesystem in general — ceremony without benefit at this size.
2. **Hyprland IPC via `hyprctl` subprocess**, not the raw socket: simpler, stable JSON with `-j`, and identical behavior to what users can debug by hand. Live changes use `hyprctl eval 'hl.monitor({...})'` — NOT `hyprctl keyword monitor`, which a Lua-configured Hyprland rejects with "keyword can't work with non-legacy parsers. Use eval." (found in real-hardware testing). This unifies the wire format: the same `hl.monitor({...})` entry serves live apply and persistence. The event socket (`$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`) is read on a background thread only for `monitoradded`/`monitorremoved` to trigger re-query (satisfies the 2s hotplug requirement).
3. **Canvas model**: work in logical pixels (resolution/scale); render at a fit-to-viewport zoom factor. Snapping: while dragging, project candidate edges against all other monitors' edges within a snap threshold (~24 canvas px); on release, resolve overlaps by pushing to the nearest snapped free position, then normalize the whole layout so the top-left bounding corner is 0x0 (Hyprland positions must be non-negative and gap-consistent).
4. **Live apply / revert — atomic batch**: before applying, snapshot the current state as `hl.monitor({...})` Lua entries. The whole layout is applied as ONE `hyprctl eval` chunk (all entries joined, disabled ones as `disabled = true`) — applying monitors one at a time creates transient overlapping states that Hyprland rejects with "Your monitor layout is set up incorrectly. Monitor X overlaps..." (found in real-hardware testing; a batched position swap returns plain "ok"). Show a 15s countdown dialog; on timeout or failure, replay the snapshot (also as one batch). Revert must not depend on the UI thread being responsive to the new layout — the countdown runs on a timer, and reapplying the snapshot is a plain subprocess call.
5. **Lua persistence via managed lines, not a Lua parser**: rewrite the file line-by-line, replacing lines that match `hl.monitor(` whose `output` is a monitor we manage; all other lines (comments, `hl.env`, the `output = ""` fallback) pass through untouched. Managed entries are written in a stable order before the fallback rule. This avoids a Lua AST dependency and survives user comments. Backup to `monitors.lua.bak` (single rolling backup) + write-to-temp-then-rename for atomicity.
6. **Mode strings**: persist `mode = "WxH@Hz"` using the exact mode chosen from `availableModes`; if the user never touched the mode, keep whatever string the file already had (e.g. `"preferred"`), so we don't pin a laptop panel unnecessarily.

7. **Config-provider detection and dual adapters**: `hyprctl systeminfo` reports `configProvider: lua` on Lua systems (verified on real hardware) — anything else (or the line's absence) selects the classic path. The `Compositor::apply_layout` port takes domain `MonitorState` values; each adapter renders its own wire format (`hl.monitor({...})` chunk for `EvalCompositor`, `hyprctl --batch "keyword monitor ...; ..."` for `KeywordCompositor`). Classic persistence targets `~/.config/hypr/monitors.conf` (the user must `source` it from hyprland.conf — documented in the README, same convention as nwg-displays). The line-rewrite algorithm (managed block replacement, backup, atomic write) is shared between the Lua and conf stores via `adapters/managed_lines.rs`.
8. **Renamed to `hyprland-monitors`**: "hyprmon" collides with existing projects (erans/hyprmon TUI, a1rb0rn3/hyprmon daemon); the tool is generic for Hyprland, so the name reflects that. UI strings are in English.

## Risks / Trade-offs

- **Line-based Lua editing** breaks if the user writes multi-line `hl.monitor({...})` calls. Accepted for v1: Omarchy's generated file is one call per line; a parse failure falls back to "replace managed block, append unknown lines unchanged" and never silently drops content.
- **Live apply vs persisted config drift**: eliminated by construction — live apply (`hyprctl eval`) and persistence write the exact same `hl.monitor({...})` entry, generated by `MonitorState::to_lua_entry()` from the same normalized state.
- **egui drag ergonomics** are hand-rolled (no ready-made snap widget). Contained risk: the `layout` module is pure geometry with unit tests; the widget only feeds it pointer deltas.
- **Auto-revert races hotplug**: if a monitor is unplugged during the countdown, replaying the snapshot may reference a gone output. Hyprland ignores rules for absent outputs, so revert degrades gracefully; we re-query state after any revert.
- **Classic path untestable on this machine**: this box runs the Lua provider, so `KeywordCompositor` and the conf store are covered by unit tests and mirror the verified Lua flow, but have not run against a real classic Hyprland. `hyprctl --batch` may still emit transient overlap warnings per keyword — flagged for validation on a classic setup.
