# ADR 0001: Independent server + desktop client

## Status
Accepted

## Context
BotFlow needs a self-hostable automation engine. Operators run the API/runtime on a server; users manage workflows from a desktop app.

## Decision
- Ship `botflow-server` as a headless Axum process (API + SSE + engine).
- Ship a **Dioxus** desktop studio (`botflow-desktop`) as the primary UI.
- This branch contains the Rust workspace only.

## Consequences
CORS and auth must support a remote UI origin when the desktop talks to a remote engine. Production does not require the server to serve a web SPA.
