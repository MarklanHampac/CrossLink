# CrossLink Desktop (Windows Host) – UX feedback & Hot Reload

## User feedback / UX (implemented)
- **Busy/progress indicator**: indeterminate progress bar + status text shown during Discover, Connect, Disconnect actions; buttons/inputs disabled while busy.
- **Explicit notifications**: success/error notice banner with clear messages for Discover found/not found, Connect success/failure (with reason/hints), Disconnect, switch-control, and unexpected disconnect reasons (timeout/peer closed).
- **Status visibility**: connection status (with peer info), active device, and color-coded indicator (yellow busy, green connected, red disconnected/error).
- **Troubleshooting hints** included in UI (same Wi-Fi, ports 8888/8889, firewall, app listening).

## Hot reload (developer experience)
See `host/DEV.md` for full steps.
- **Logic hot-restart**: `cargo install cargo-watch` then `cd host && cargo watch -x run` (or `host/dev-watch.cmd`). Rebuilds and restarts on Rust/UI-binding changes.
- **Slint UI live-preview**: run with `cargo run --features live-preview` and use Slint VS Code extension / `slint-lsp` live preview for `host/app.slint` to iterate on UI faster.

## Connection troubleshooting notes
- Ensure phone app is open/listening and Accessibility/Keyboard/Overlay enabled as needed.
- Same LAN/subnet, no guest isolation; allow TCP 8888 and UDP 8889 in Windows Firewall (Private).
- Use Discover or enter `PHONE_IP:8888` shown in Android app.
