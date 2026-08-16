# config-style-detection

## Purpose

Makes the tool generic across Hyprland setups by detecting which config provider the running compositor uses, so live apply and persistence always match the user's config style.

## ADDED Requirements

### Requirement: Detect the active config provider
The system SHALL detect at startup whether the running Hyprland uses the Lua config provider or the classic (.conf) provider, and select the matching live-apply mechanism and persistence format.

#### Scenario: Lua provider (e.g. Omarchy)
- **WHEN** the app starts on a Hyprland whose `hyprctl systeminfo` reports `configProvider: lua`
- **THEN** live changes use Lua evaluation and persistence targets `monitors.lua`

#### Scenario: Classic provider
- **WHEN** the app starts on a Hyprland that does not report the Lua config provider
- **THEN** live changes use the keyword mechanism and persistence targets `monitors.conf`
