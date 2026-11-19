# Agent notes

Flowlist is a local-first task manager written in Rust with an official GPUI
desktop window, a CLI and an MCP server over one shared `flowlist-core`.

## Before starting

- **Read [`docs/backlog.md`](docs/backlog.md).** It is the durable source of
  truth for outstanding work, including the Things 3 "Pending" list. A context
  reset is not a reason to lose it; update the file as items move.
- Check `docs/architecture.md` and `docs/features.md` for what already exists.
- Keep the working tree clean and push after each major milestone.

## Non-negotiables

- Pure Rust. No web view, Tauri or Electron. Use the official `gpui` crate.
- The GUI, CLI and MCP server share `flowlist-core`; never fork the domain
  logic into a UI-only copy.
- Every mutation goes through the command/store layer so it is undoable.
- Keyboard-first: vim bindings cover the whole app and macOS shortcuts remain.
- Icon-only controls need a delayed tooltip and a keyboard route.
- Text must stay legible in all ten theme families and both appearances. Keep
  the contrast contract in `crates/flowlist/src/gui/theme.rs` green.
- Never print the user's email address in Git output or replies.

## Checks

```sh
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo build --release
scripts/gui-smoke.sh
scripts/gui/forecast-probe.sh
```

The GUI smoke test needs a display and uses `FLOWLIST_TEST_STATE`; see
`docs/gui-automation.md`. After UI changes, update `docs/features.md`,
`docs/accessibility.md` and `CHANGELOG.md`.
