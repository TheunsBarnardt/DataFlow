# ADR 0002: MongoDB persistence

## Status
Accepted

## Context
The TypeScript BotFlow already uses MongoDB (`botflow` database) with collections for workflows, runs, credentials, etc.

## Decision
Use the official `mongodb` Rust driver. Prefer compatible document shapes so Node and Rust can share a database during comparison. Do not introduce SQLite/PostgreSQL as the primary store in this migration.

## Consequences
Repository interfaces live in `crates/storage`. Indexes mirror `server/src/db.ts`.
