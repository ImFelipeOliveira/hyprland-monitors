# layout-editor Specification

## Purpose
The visual heart of the app: a canvas where monitors are arranged by dragging proportional rectangles, plus per-monitor settings, replacing hand-editing of coordinates.

## Requirements

### Requirement: Canvas renders monitors proportionally
The system SHALL render each monitor as a rectangle on a canvas, scaled proportionally to its logical size (resolution divided by scale), positioned according to the current layout, and labeled with its output name and description.

#### Scenario: Initial render matches current layout
- **WHEN** the application opens with HDMI-A-1 at 0x0 and eDP-1 at 0x1080
- **THEN** the canvas shows the HDMI rectangle directly above the eDP rectangle, sized proportionally to their logical resolutions

### Requirement: Drag-and-drop positioning with snapping
The system SHALL let the user reposition a monitor by dragging its rectangle. While dragging, edges SHALL snap to adjacent monitors' edges (making above/below/left/right placement effortless), and on release the layout SHALL contain no overlapping monitors and no gaps introduced by rounding.

#### Scenario: Move external monitor to the right of the laptop
- **WHEN** the user drags the HDMI-A-1 rectangle from above eDP-1 to its right side and releases near its right edge
- **THEN** the rectangle snaps flush to eDP-1's right edge and the resulting positions are HDMI-A-1 at 1920x0-relative alignment with no overlap

#### Scenario: Drop causing overlap is corrected
- **WHEN** the user releases a dragged monitor overlapping another monitor's rectangle
- **THEN** the system resolves the position to the nearest non-overlapping snapped placement

### Requirement: Per-monitor settings
The system SHALL allow the user to select a monitor and change its resolution + refresh rate (from the monitor's available modes), its scale, and its enabled state. Changing mode or scale SHALL immediately update the rectangle's proportional size on the canvas.

#### Scenario: Change refresh rate
- **WHEN** the user selects eDP-1 and picks 1920x1080@144 from the mode list
- **THEN** the candidate layout records that mode for eDP-1

#### Scenario: Disable a monitor
- **WHEN** the user toggles a monitor to disabled
- **THEN** its rectangle is shown visually muted and it is excluded from position snapping of other monitors

### Requirement: Last enabled monitor cannot be disabled
The system SHALL prevent disabling a monitor when it is the only enabled monitor in the candidate layout.

#### Scenario: Attempt to disable the only monitor
- **WHEN** only eDP-1 is enabled and the user tries to disable it
- **THEN** the toggle is rejected and the UI explains that at least one monitor must stay enabled
