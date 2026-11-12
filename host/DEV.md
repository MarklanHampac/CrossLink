# CrossLink Host - Development & Hot Reload

## Hot reload workflows

1. **Rust logic hot-restart (recommended quick loop)**
   - Install: `cargo install cargo-watch`
   - Run: `cd host && cargo watch -x run` (or `dev-watch.cmd` on Windows)
   - Rebuilds and restarts app on source changes.

2. **Slint UI live-preview (hot reload UI without full restart)**
   - Install Slint VS Code extension and/or `cargo install slint-lsp`.
   - Build/run with live-preview feature: `cargo run --features live-preview`
   - Use Slint live-preview tooling to preview/edit `app.slint` with live updates; compiled app still uses `slint::include_modules!()` for release.
   - For design-time preview of the `.slint` file directly: `slint-viewer app.slint` (if installed).

## Notes
- Release builds hide console (`windows_subsystem = "windows"`). Debug builds show logs.
- Network actions now report busy/progress and explicit success/error notices in UI.
