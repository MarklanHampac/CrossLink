@echo off
REM Dev hot-reload workflow for CrossLink host UI/logic
REM Requires: cargo install cargo-watch (for rebuild/restart on code changes)
REM UI live-preview: run with Slint live-preview tools (e.g. VS Code Slint extension) and: cargo run --features live-preview
echo Starting CrossLink host with cargo-watch (rebuild+restart on changes)...
cargo watch -x "run"
