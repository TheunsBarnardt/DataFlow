# Architecture notes (Rust)

This branch ships the **Rust** stack only (`crates/`).

| Concern | Implementation |
|---------|----------------|
| HTTP | Axum (`botflow-app` / embedded in desktop) |
| DB | MongoDB |
| UI | Dioxus 0.7 + dioxus-flow + shadcn tokens (native desktop) |
| Auth | JWT Bearer (`BOTFLOW_AUTH_DISABLED=1` for local) |
| Engine | `runtime-scheduler` + `nodes-core` |
| Nodes | Catalog-registered executors in `nodes-core` |

## Run

```powershell
.\scripts\run-desktop.ps1
```
