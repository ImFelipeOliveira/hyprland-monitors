# config-persistence (delta)

## ADDED Requirements

### Requirement: Advanced fields are persisted
Persisted entries SHALL include transform, VRR and mirror whenever they differ from the defaults, in the format of the active provider (Lua `transform = N` / `vrr = N` / `mirror = "X"`; classic `, transform, N` / `, vrr, N` / `, mirror, X`). Default values SHALL be omitted so untouched configs stay byte-identical to v0.1.0 output.

#### Scenario: Persist rotation on Lua provider
- **WHEN** the user keeps a layout where eDP-1 has transform 3
- **THEN** monitors.lua contains `transform = 3` inside eDP-1's `hl.monitor({...})` entry

#### Scenario: Defaults are omitted
- **WHEN** a monitor has transform 0, VRR off and no mirror
- **THEN** its persisted entry contains none of the new fields
