# ADR 0003: In-process execution first

## Status
Accepted

## Context
Distributed workers are a later scale option.

## Decision
Execute workflows in-process inside `botflow-server` with bounded concurrency. Extract `botflow-worker` later using the same job/execution contracts (M11).

## Consequences
Runtime crates must not depend on HTTP or UI. Job lease APIs are designed early even if only local execution is used initially.
