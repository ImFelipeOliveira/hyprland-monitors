# monitor-detection

## Purpose

Provides an accurate, up-to-date view of all monitors known to the running Hyprland compositor, so the layout editor always starts from the real hardware state.

## ADDED Requirements

### Requirement: Enumerate connected monitors
The system SHALL enumerate all monitors known to the running Hyprland compositor, including disabled ones, and expose for each: output name, human-readable description (make/model), available modes (resolution + refresh rate), current mode, position, scale, transform, and enabled state.

#### Scenario: Two monitors connected
- **WHEN** the application starts on a system with a laptop panel (eDP-1) and an external monitor (HDMI-A-1)
- **THEN** both monitors are listed with their names, descriptions, available modes, and current position/scale/enabled state

#### Scenario: Disabled monitor present
- **WHEN** a connected monitor is currently disabled in Hyprland
- **THEN** it still appears in the enumeration, marked as disabled, with its available modes

### Requirement: Refresh on hotplug
The system SHALL update the monitor list when monitors are connected or disconnected while the application is running, without requiring a restart.

#### Scenario: Monitor plugged in while app is open
- **WHEN** the user connects a new monitor via HDMI while the application is running
- **THEN** the new monitor appears in the UI within 2 seconds

### Requirement: Fail clearly outside Hyprland
The system SHALL detect when it cannot communicate with a running Hyprland instance and present a clear error message instead of an empty or broken UI.

#### Scenario: Launched outside Hyprland
- **WHEN** the application is launched in an environment where the Hyprland IPC is unavailable
- **THEN** the application shows an error explaining that a running Hyprland session is required and exits gracefully
