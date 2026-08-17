# Proposal: advanced-display-features

## Why

v0.1.0 covers position, mode, scale and enable/disable — but rotated monitors (vertical coding screens), mirrored presentations, VRR gaming setups and dock/undock routines are everyday needs the roadmap already promised. Users currently fall back to hand-editing for exactly these.

## What Changes

- **Transform/rotation**: per-monitor rotation (90°/180°/270°) and flipped variants, reflected proportionally on the canvas (rotated monitors render with swapped width/height).
- **Mirroring**: a monitor can mirror another's content (`mirror` field), selectable in the settings panel.
- **VRR**: per-monitor variable refresh rate toggle (off / on / fullscreen-only).
- **Named profiles**: save the current layout under a name and load it later (e.g. "docked", "mobile"); loading fills the candidate layout, which the user applies with the existing confirm/revert flow.
- All new fields flow through the existing atomic live apply and config persistence for BOTH providers (Lua and classic).

## Capabilities

### New Capabilities

- `layout-profiles`: save, list, load and delete named monitor layouts, stored outside the Hyprland config.

### Modified Capabilities

- `layout-editor`: per-monitor settings gain transform, mirror and VRR; canvas reflects rotation in monitor proportions.
- `live-apply`: applied entries include transform, VRR and mirror.
- `config-persistence`: persisted entries include transform, VRR and mirror in both formats.

## Impact

- `domain/monitor.rs` (new fields + wire rendering), `application/session.rs` (setters + profile use cases), new `ProfileStore` port + JSON adapter, settings panel and toolbar UI.
- New user file: `~/.config/hyprland-monitors/profiles.json`.
- No breaking changes; monitors without the new fields render exactly as before.
