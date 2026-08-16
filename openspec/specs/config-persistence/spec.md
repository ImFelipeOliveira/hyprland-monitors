# config-persistence Specification

## Purpose
Makes a confirmed layout survive reboots by writing it to Omarchy's Lua-based Hyprland config, without destroying anything else the user keeps in that file.

## Requirements

### Requirement: Persist layout in Omarchy Lua format
The system SHALL write the confirmed layout to `~/.config/hypr/monitors.lua` as one `hl.monitor({ output = ..., mode = ..., position = ..., scale = ... })` entry per monitor, producing a file Hyprland/Omarchy loads successfully at startup. Disabled monitors SHALL be persisted as disabled.

#### Scenario: Persist a two-monitor layout
- **WHEN** the user confirms a layout with HDMI-A-1 (1920x1080@144 at 0x0) and eDP-1 (preferred at 0x1080)
- **THEN** `monitors.lua` contains matching `hl.monitor({...})` entries and the next Hyprland startup reproduces the layout

### Requirement: Preserve non-monitor content
The system SHALL preserve content in `monitors.lua` that is not an `hl.monitor(...)` entry it manages — such as env-var lines, comments, and the generic fallback rule (`output = ""`) — when rewriting the file.

#### Scenario: File contains env var and fallback rule
- **WHEN** the existing file sets `GDK_SCALE` and ends with a fallback `hl.monitor({ output = "", ... })` rule
- **THEN** after persisting, both are still present and the fallback rule remains after the managed entries

### Requirement: Persist in classic format on classic systems
On a Hyprland using the classic .conf provider, the system SHALL write the confirmed layout to `~/.config/hypr/monitors.conf` as one `monitor = <name>, <mode>, <pos>, <scale>` line per monitor (disabled monitors as `monitor = <name>, disable`), preserving unrelated lines and any generic fallback rule, with the same backup and atomic-write guarantees.

#### Scenario: Persist on classic Hyprland
- **WHEN** the user confirms a layout on a classic-configured Hyprland
- **THEN** `monitors.conf` contains matching `monitor = ...` lines, comments and unrelated lines survive, and a `.bak` backup of the previous content exists

### Requirement: Backup before write
The system SHALL create a backup copy of the existing `monitors.lua` before each write, and SHALL leave the original untouched if writing fails partway.

#### Scenario: Write fails mid-way
- **WHEN** writing the new file fails (e.g., disk full)
- **THEN** the original `monitors.lua` remains intact and the user is told persistence failed while the running session keeps the applied layout
