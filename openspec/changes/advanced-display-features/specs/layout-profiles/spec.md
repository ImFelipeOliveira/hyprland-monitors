# layout-profiles

## Purpose

Lets users save named monitor layouts (e.g. "docked", "mobile") and reload them later, so recurring hardware setups are one load + apply away instead of re-dragging.

## ADDED Requirements

### Requirement: Save current layout as a named profile
The system SHALL save the current candidate layout (per monitor: mode, position, scale, enabled, transform, VRR, mirror) under a user-chosen name, stored outside the Hyprland config files. Saving to an existing name SHALL overwrite it.

#### Scenario: Save a docked profile
- **WHEN** the user names the current layout "docked" and saves
- **THEN** the profile is persisted and appears in the profile list on the next app start

### Requirement: Load a profile into the candidate layout
Loading a profile SHALL update the candidate layout for the connected monitors it covers, leaving monitors absent from the profile untouched, and SHALL NOT apply anything to the compositor by itself — the user applies via the existing confirm/revert flow.

#### Scenario: Load with a missing monitor
- **WHEN** the user loads a profile referencing DP-1 while DP-1 is not connected
- **THEN** the entries for connected monitors are loaded, DP-1's entry is ignored, and no error blocks the load

### Requirement: Delete a profile
The system SHALL let the user delete a saved profile by name.

#### Scenario: Delete
- **WHEN** the user deletes "docked"
- **THEN** it disappears from the profile list and from the storage file
