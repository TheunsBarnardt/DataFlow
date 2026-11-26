# MVP scope

## Nodes (must)
Manual Trigger, Webhook Trigger, HTTP Request, Set/Transform, IF/Branch, Merge

## Nodes (soon)
Wait, Schedule

## Demo workflow
Manual Trigger → Set Data → (Parallel HTTP A + HTTP B) → Merge → IF → Output

## First usable release
- Independent `botflow-server` + MongoDB
- Downloadable / Tauri UI manages server via configured URL
- Create/validate/save/publish/import/export
- Core nodes, bounded parallelism, SSE, persisted runs
- Timeout/retry/cancel visible; credentials redacted

## Comparison
Branch `typesctipt` = TypeScript stack. Branch `rust` = TypeScript kept + Rust crates. Point UI at either API via `VITE_API_BASE_URL`.
