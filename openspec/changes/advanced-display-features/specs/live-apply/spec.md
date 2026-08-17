# live-apply (delta)

## ADDED Requirements

### Requirement: Advanced fields are applied live
Applied entries SHALL include the monitor's transform, VRR mode and mirror source whenever they differ from the defaults (transform 0, VRR off, no mirror), on both config providers.

#### Scenario: Apply a rotated monitor
- **WHEN** the user applies a layout where eDP-1 has transform 1
- **THEN** the entry sent to the compositor includes the transform and the running session shows the monitor rotated
