# Tasks: advanced-display-features

## 1. Domain

- [x] 1.1 `MonitorState`: `transform`, `vrr`, `mirror_of` fields; `from_raw` reads `transform`/`vrr`/`mirrorOf`; `logical_size` swaps dims on odd transforms
- [x] 1.2 Wire renderers (Lua entry, keyword arg, conf line) append non-default fields; unit tests incl. byte-identical default output

## 2. Application

- [x] 2.1 Session setters: `set_transform`, `set_vrr`, `set_mirror` (self-mirror rejected), all normalizing
- [x] 2.2 `ProfileStore` port + `Profile` serde types; Session: `profile_names`, `save_profile`, `load_profile` (name-matching, mode pinned as touched, unmatched monitors untouched), `delete_profile`; unit tests with in-memory store

## 3. Adapters

- [x] 3.1 `JsonProfileStore` at `~/.config/hyprland-monitors/profiles.json` (atomic write); round-trip test in temp dir

## 4. UI

- [x] 4.1 Settings panel: transform, VRR and mirror comboboxes (mirror list excludes self)
- [x] 4.2 Canvas: "mirrors X" subtitle for mirrored monitors; rotation reflected via logical size
- [x] 4.3 Toolbar profiles cluster: profile list (load on select), name field, save, delete

## 5. Wrap-up

- [x] 5.1 README: features + roadmap checkboxes; tests green, clippy clean; reinstall local binary
- [ ] 5.2 Manual validation on real hardware (rotation apply/revert, profile save/load) — user
