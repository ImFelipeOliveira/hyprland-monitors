# layout-editor (delta)

## ADDED Requirements

### Requirement: Transform, mirror and VRR settings
The system SHALL let the user set, per monitor: a transform (normal, 90°, 180°, 270°, and flipped variants), a mirror source (none or another connected monitor), and a VRR mode (off, on, fullscreen-only).

#### Scenario: Rotate a monitor
- **WHEN** the user sets a monitor's transform to 90°
- **THEN** the candidate layout records transform 1 and the canvas rectangle swaps width and height proportionally

#### Scenario: Mirror another monitor
- **WHEN** the user sets HDMI-A-1 to mirror eDP-1
- **THEN** the candidate records the mirror source and the canvas labels HDMI-A-1 as mirroring eDP-1

#### Scenario: Mirror choices exclude self
- **WHEN** the user opens the mirror selector for eDP-1
- **THEN** eDP-1 itself is not offered as a mirror source
