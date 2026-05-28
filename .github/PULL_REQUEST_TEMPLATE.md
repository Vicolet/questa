## What changes

<!-- One or two sentences. The reader should know what to expect from the diff. -->

## Why

<!-- Motivation. A linked issue is fine ("closes #42") if the issue body has the why. -->

## Docs checklist

Drop the items that genuinely don't apply, tick the rest:

- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] `README.md` keybindings table updated (if a key was added, removed, or rebound)
- [ ] `docs/schema.md` updated (if a JSON field was added, removed, renamed, or its edit path changed)
- [ ] Help overlay in `src/ui.rs::draw_help_overlay` updated (if a key changed)
- [ ] New tests covering the change (unit, snapshot, or both)
- [ ] `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check` all pass locally
