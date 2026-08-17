# Design: advanced-display-features

## Context

Hyprland's `HL.MonitorSpec` already accepts `transform` (0–7), `vrr` (0/1/2) and `mirror` (output name); the classic keyword takes the same as `, transform, N` / `, vrr, N` / `, mirror, X` suffixes. `hyprctl -j monitors all` reports `transform`, `vrr` (bool) and `mirrorOf` per monitor, so runtime state remains the single source of truth.

## Goals / Non-Goals

**Goals:** transform, mirror, VRR editable per monitor; named profiles; everything flowing through the existing atomic apply + revert + persistence for both providers.

**Non-Goals:** automatic profile switching on hotplug (kanshi-style daemon), fractional custom transforms, per-profile wallpaper/workspace rules.

## Decisions

1. **Domain fields, not new types**: `MonitorState` gains `transform: u8`, `vrr: u8`, `mirror_of: Option<String>`. `logical_size()` swaps width/height for odd transforms (90°/270°), so the canvas, snapping and normalization pick up rotation with zero changes to `geometry.rs`. Wire renderers append the fields only when non-default, keeping v0.1.0 output byte-identical for untouched monitors.
2. **Runtime as source of truth**: fields are read back from `hyprctl -j` after apply/resync (`transform`, `vrr` bool → 0/1, `mirrorOf`). Known limitation: VRR mode 2 (fullscreen-only) reads back as on/off; documented, acceptable.
3. **Profiles via a third port**: `ProfileStore` (list/load/save/delete) keeps the session testable with an in-memory fake. Real adapter stores a single JSON map at `~/.config/hyprland-monitors/profiles.json` (serde). Loading matches monitors by output name, marks loaded modes as touched (profiles pin modes deliberately), leaves unmatched monitors untouched, then normalizes. Loading never touches the compositor — apply stays an explicit user action guarded by the countdown.
4. **UI placement**: transform/VRR/mirror as comboboxes in the existing settings panel; profiles as a toolbar cluster (list + name field + save/delete). Mirrored monitors show a "mirrors X" subtitle on the canvas.

## Risks / Trade-offs

- Mirroring semantics in Hyprland ignore the mirrored monitor's position; we still keep its rect on the canvas (labeled) rather than hiding it — simpler and reversible.
- `hyprctl --batch` classic path gains the same suffixes; still pending real-hardware validation (pre-existing caveat).
