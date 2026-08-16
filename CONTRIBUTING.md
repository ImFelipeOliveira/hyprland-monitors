# Contributing to hyprland-monitors

Thanks for your interest! This document explains how the project works and how
to get a change merged.

## Development setup

Requirements: Rust (stable), a running Hyprland session for manual testing.

```sh
git clone https://github.com/ImFelipeOliveira/hyprland-monitors
cd hyprland-monitors
cargo test                                   # unit tests (no compositor needed)
cargo clippy --all-targets -- -D warnings    # must be clean
cargo fmt --all                              # rustfmt formatting
cargo run                                    # needs a live Hyprland session
```

The whole apply/confirm/revert/persist flow is unit-tested with in-memory fakes,
so most changes can be developed and verified without touching a real compositor.

## Architecture in 30 seconds

Light ports & adapters; the directory tree mirrors the layers (see the README's
Architecture section for the full map):

- `src/domain/` — pure logic (geometry, monitor types, wire-format rendering). No IO.
- `src/application/` — `ports.rs` (the `Compositor` and `ConfigStore` traits) and
  `session.rs` (the state machine). Depends only on the ports.
- `src/adapters/` — real implementations: `hyprctl.rs`, `omarchy_lua.rs`,
  `hyprland_conf.rs`, plus the shared `managed_lines.rs` rewrite core.
- `src/ui/` — thin egui view. No business logic here, ever.

Rules of thumb: new IO goes behind a port; new logic goes in `domain`/`application`
with tests; the UI only renders state and forwards intents to the `Session`.

## Spec-driven workflow (OpenSpec)

Behavior is specified before it is implemented. The living specs are in
`openspec/specs/` (one folder per capability), managed with
[OpenSpec](https://github.com/Fission-AI/OpenSpec):

- Changing spec-level behavior (new feature, changed requirement) starts with an
  OpenSpec change: `openspec new change <name>`, fill in proposal → specs →
  design → tasks, then implement. The change is archived on completion.
- Pure refactors, docs, and bug fixes that don't change specified behavior don't
  need a spec change.

If that sounds heavy: for small features it's ~2 short markdown files, and it's
also the fastest way to get maintainer buy-in before you write code.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org) —
[release-please](https://github.com/googleapis/release-please) turns them into
version bumps, CHANGELOG entries and GitHub releases automatically:

- `feat: ...` → minor bump
- `fix: ...` → patch bump
- `feat!: ...` or a `BREAKING CHANGE:` footer → major bump
- `chore:`, `docs:`, `refactor:`, `test:`, `ci:` → no release

## Pull requests

1. Fork, create a branch, commit with conventional messages.
2. Make sure CI passes locally: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test`.
3. Say in the PR description which config provider you tested on (Lua/Omarchy or
   classic .conf) — the classic path especially needs real-hardware reports.

## Reporting bugs

Use the bug report template — the Hyprland version and `configProvider` matter a
lot. Never include your monitor serials if you're not comfortable sharing them.
