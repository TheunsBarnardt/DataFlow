# Extensions and sandboxing (M12)

## Goals
Allow third-party or user-defined nodes without giving them unrestricted runtime authority.

## Feasibility notes
- **In-process Rust plugins** (dynamic libraries): fast, hard to isolate; not recommended for untrusted code.
- **WASM components**: promising isolation (fuel/memory limits via wasmtime). Node SDK would compile to WASM with host imports for HTTP/credentials gated by allowlists.
- **Sidecar processes**: language-flexible; IPC + resource limits via OS; higher latency.

## Recommended path
1. Keep first-party nodes as Rust crates (`nodes-core`, `integrations`).
2. Prototype WASM node ABI: `execute(ctx_json) -> result_json` with host functions for `http_fetch`, `get_credential` behind permission flags.
3. Signing/trust: signed manifests; admin allowlist of publisher keys.
4. Version compatibility: node metadata `version` + engine min version.

## Permission model
- Default deny network, filesystem, credentials.
- Capabilities declared in node manifest; checked at registration and at each host call.
- Budgets: max CPU fuel, wall timeout, max response bytes.

## Status
Documented for M12; WASM host not shipped yet. Isolation model above is the acceptance baseline.
