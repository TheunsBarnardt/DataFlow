# Production readiness (M13)

## Binaries
- `botflow` (`botflow-desktop`) — desktop app: native window + embedded engine
- `botflow-server` (`botflow-app`) — headless API / engine only
- `botflow-worker` — optional leased job worker

## Health
`GET /api/v1/health` (and `/api/health`) on the local engine port

## Metrics / tracing
Set `RUST_LOG=info,botflow=debug`. Export via `OTEL` can be added; audit events land in Mongo `audit_events`.

## Backup / restore
Backup MongoDB (`mongodump` / Atlas snapshots). Workflow JSON export from UI for portable definitions. Credentials require `CREDENTIALS_KEY` to decrypt.

## Limits
- `MAX_CONCURRENT_NODES` (default 8)
- HTTP SSRF guard (`HTTP_ALLOW_PRIVATE=false` in prod)
- LLM allowlist `LLM_MODEL_ALLOWLIST`

## Install (Windows desktop)
```powershell
cargo install tauri-cli --version "^2" --locked
cd crates\desktop
cargo tauri build
```

Installer targets: NSIS / MSI under `target/release/bundle`.

## Headless engine
```powershell
cargo build --release -p botflow-app
$env:MONGODB_URI="mongodb://127.0.0.1:27017/botflow"
$env:PORT="8788"
.\target\release\botflow-server.exe
```

## Stack
| Binary | Start | Notes |
|--------|-------|-------|
| Desktop | `dx serve --desktop` (dev) / `cargo run -p botflow-desktop` | Native app + loopback engine |
| Headless | `cargo run -p botflow-app` | API (default 8788) |
| Worker | `cargo run -p botflow-worker` | Optional leased jobs |
