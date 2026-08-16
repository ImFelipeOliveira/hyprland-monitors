# Tasks: hyprland-monitor-ui

## 1. Project scaffolding

- [x] 1.1 Create Rust binary crate (`cargo init`) with `eframe`, `egui`, `serde`, `serde_json` dependencies; module skeleton `hypr`, `model`, `layout`, `lua`, `ui`
- [x] 1.2 Empty eframe window opens under Hyprland with app title and closes cleanly

## 2. Monitor detection (`monitor-detection`)

- [x] 2.1 `hypr::query()` — run `hyprctl -j monitors all`, deserialize into `model::Monitor` (name, description, modes, current mode, position, scale, enabled)
- [x] 2.2 Clear error path when `hyprctl` is missing or IPC unavailable (dialog + graceful exit)
- [x] 2.3 Background thread on Hyprland event socket: on `monitoradded`/`monitorremoved`, re-query and notify the UI (≤2s)
- [x] 2.4 Unit tests for deserialization against captured `hyprctl -j` fixtures

## 3. Layout geometry (`layout-editor` core)

- [x] 3.1 `model::Layout` in logical pixels; conversion from detected state; normalization to non-negative origin
- [x] 3.2 Edge snapping: candidate position → snapped position given other monitors and threshold
- [x] 3.3 Overlap resolution on drop: nearest non-overlapping snapped placement
- [x] 3.4 Unit tests: above/below/left/right snaps, overlap pushes, single-monitor, disabled monitors excluded

## 4. Canvas UI (`layout-editor`)

- [x] 4.1 Canvas widget: proportional rectangles, fit-to-viewport zoom, name + description labels, selection highlight
- [x] 4.2 Drag handling wired to snapping/overlap logic, with live snap guides while dragging
- [x] 4.3 Settings panel for selected monitor: mode dropdown (from availableModes), scale input, enable/disable toggle; canvas resizes rectangle on change
- [x] 4.4 Guard: refuse disabling the last enabled monitor, with explanatory tooltip/message
- [x] 4.5 Visual state for disabled monitors (muted, excluded from snapping)

## 5. Live apply (`live-apply`)

- [x] 5.1 Snapshot current state as revert keyword strings; `hypr::apply(layout)` emitting `hyprctl keyword monitor` per monitor (incl. `,disable`)
- [x] 5.2 Confirm dialog with ≥10s countdown; timeout or "Revert" replays snapshot; "Keep" promotes candidate to current
- [x] 5.3 Per-monitor failure surfacing: name the monitor/setting that was rejected, then revert
- [x] 5.4 Re-query state after any apply/revert so UI matches reality

## 6. Persistence (`config-persistence`)

- [x] 6.1 `lua::parse` — classify lines of `monitors.lua` (managed `hl.monitor` lines vs passthrough incl. `hl.env`, comments, `output = ""` fallback)
- [x] 6.2 `lua::serialize` — rewrite managed entries from confirmed layout, keep `"preferred"` when the mode was untouched, keep fallback rule last
- [x] 6.3 Rolling backup `monitors.lua.bak` + atomic temp-file-then-rename write; report failure without corrupting original
- [x] 6.4 Round-trip unit tests using the real Omarchy file shape (env var + comments + fallback)
- [x] 6.5 "Keep settings" flow offers/performs persistence and confirms the file was written

## 7. Polish and release

- [x] 7.1 `.desktop` entry + icon so the app appears in Omarchy's launcher
- [x] 7.2 Manual end-to-end test on real hardware: validated by Felipe on Omarchy (Lua provider), 2026-08-16
- [x] 7.3 README with install instructions (`cargo install --path .`); screenshots pending first real-hardware run

## 8. Generic Hyprland support (`config-style-detection`) + rename

- [x] 8.1 Rename project/binary to `hyprland-monitors` (Cargo.toml, .desktop, README, installed binary); translate all UI strings to English
- [x] 8.2 Port refactor: `Compositor::apply_layout` takes `&[MonitorState]`; wire-format rendering moves into adapters
- [x] 8.3 `detect_provider()` via `hyprctl systeminfo` (`configProvider: lua` → Lua, otherwise classic)
- [x] 8.4 `KeywordCompositor`: atomic apply via `hyprctl --batch "keyword monitor ...; ..."`
- [x] 8.5 `ConfConfigStore`: monitors.conf rewrite (managed lines, preferred-mode reuse, fallback rule, backup + atomic write), sharing the line-rewrite core with the Lua store
- [x] 8.6 Wire provider detection into App startup; unit tests for conf round-trip and keyword rendering
- [x] 8.7 Manual validation on a classic-configured Hyprland — deferred: no classic machine available; documented as a known caveat in README and design.md (unit-tested, mirrors the validated Lua flow)
