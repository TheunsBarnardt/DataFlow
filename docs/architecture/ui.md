# Management UI

BotFlow desktop is a **native Dioxus 0.7 studio** (shadcn-style tokens, sidebar, panels, dioxus-flow canvas).

| Layer | Implementation |
|-------|----------------|
| Window | Dioxus desktop (`tao`/`wry` under the hood) |
| UI | **Dioxus** + Tailwind utilities |
| Canvas | `dioxus-flow` |
| Engine | Rust Axum + MongoDB on loopback |

## Run

```powershell
$env:BOTFLOW_AUTH_DISABLED='1'
$env:MONGODB_URI='mongodb://127.0.0.1:27017/botflow'
cd crates\desktop
dx serve --desktop -p botflow-desktop
```

Or: `.\scripts\run-desktop.ps1` (auto-restarts on every save; set `BOTFLOW_HOT_RELOAD=1` for RSX hot reload, or `BOTFLOW_HOT_PATCH=1` for Rust hot-patch)
