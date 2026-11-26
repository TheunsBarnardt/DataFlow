# BotFlow

Rust engine + **Dioxus desktop studio** (dioxus-flow canvas, shadcn-style UI).

## Run

```powershell
$env:CARGO_HTTP_CHECK_REVOKE='false'
$env:BOTFLOW_AUTH_DISABLED='1'
$env:MONGODB_URI='mongodb://127.0.0.1:27017/botflow'
cd crates\desktop
dx serve --desktop -p botflow-desktop
```

Or: `.\scripts\run-desktop.ps1` (installs `dx` if missing). RSX hot-reloads on save. For experimental Rust logic hot-patching: `$env:BOTFLOW_HOT_PATCH='1'` (needs MSVC lib paths on Windows). Press `r` in the `dx` TUI for a full rebuild.

Headless API server:

```powershell
.\scripts\run-rust-server.ps1
```

## Docs

- [docs/architecture/ui.md](docs/architecture/ui.md)
- [docs/architecture/production.md](docs/architecture/production.md)
