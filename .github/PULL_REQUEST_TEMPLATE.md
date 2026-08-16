## What does this PR do?

<!-- Short description. Link the issue it closes, e.g. "Closes #12". -->

## Checklist

- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org) (`feat:`, `fix:`, `chore:`, ...) — releases are generated from them
- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --all` applied
- [ ] Spec-level behavior changes have a matching OpenSpec change under `openspec/` (see CONTRIBUTING.md)
- [ ] Tested on a real Hyprland session (state which config provider: Lua or classic)
