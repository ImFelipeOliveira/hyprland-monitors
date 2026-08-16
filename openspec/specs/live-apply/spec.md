# live-apply Specification

## Purpose
Applies a candidate layout to the running compositor immediately, with an automatic revert safety net so a wrong configuration can never leave the user without a usable screen.

## Requirements

### Requirement: Apply candidate layout to running compositor
The system SHALL apply the candidate layout to the running Hyprland session at the user's request, without restarting the compositor, so the change takes effect immediately.

#### Scenario: Apply new arrangement
- **WHEN** the user clicks "Apply" after moving HDMI-A-1 to the right of eDP-1
- **THEN** the running session immediately reflects the new arrangement on the physical screens

### Requirement: Timed confirmation with automatic revert
After applying, the system SHALL show a confirmation dialog with a visible countdown (at least 10 seconds). If the user does not confirm before the countdown ends, the system SHALL restore the previous working layout in the running session.

#### Scenario: User confirms
- **WHEN** the user clicks "Keep settings" within the countdown
- **THEN** the applied layout becomes the current layout and persistence is offered/performed

#### Scenario: User does not confirm
- **WHEN** the countdown expires with no user action (e.g., the new layout left all screens unusable)
- **THEN** the previous layout is reapplied to the running session and the candidate is discarded

### Requirement: Atomic application on both config providers
The system SHALL apply the whole layout as a single request to the compositor, never one monitor at a time (sequential application creates transient overlapping states the compositor rejects), using the mechanism matching the detected config provider.

#### Scenario: Swap positions on a Lua-configured Hyprland
- **WHEN** the user applies a layout that swaps two monitors' positions on a Lua-configured Hyprland
- **THEN** the layout is applied as one Lua chunk and the compositor accepts it without overlap errors

#### Scenario: Apply on a classic-configured Hyprland
- **WHEN** the user applies a layout on a Hyprland using the classic .conf provider
- **THEN** the layout is applied as one batched keyword request

### Requirement: Apply failures are surfaced
If the compositor rejects any part of the layout, the system SHALL report which monitor/setting failed and restore the previous layout.

#### Scenario: Invalid mode rejected
- **WHEN** applying a mode the compositor refuses
- **THEN** the user sees an error naming the monitor and setting, and the previous layout is restored
