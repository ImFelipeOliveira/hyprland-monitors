# Security Policy

## Supported versions

Only the latest release is supported with security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Instead, use
GitHub's private vulnerability reporting:

https://github.com/ImFelipeOliveira/hyprland-monitors/security/advisories/new

You should receive a response within a week. Please include reproduction steps
and the affected version.

## Scope notes

hyprland-monitors runs unprivileged, talks only to the local Hyprland IPC via
`hyprctl`, and writes only to `~/.config/hypr/monitors.lua` or
`~/.config/hypr/monitors.conf` (with a `.bak` backup). It makes no network
requests. Reports about config-file corruption, injection through crafted
monitor names/descriptions, or privilege boundary issues are especially welcome.
