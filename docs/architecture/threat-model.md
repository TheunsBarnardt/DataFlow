# Threat model (draft)

## Assets
- Credential secrets, workflow definitions, run I/O, session tokens, MongoDB data.

## Trust boundaries
- Downloadable UI ↔ API (HTTPS, CORS allowlist, session/JWT).
- Runtime nodes ↔ external network (SSRF controls on HTTP).
- Workflow JSON must not embed raw secrets (secret references only).

## Controls
- Encrypt credentials at rest (AES-GCM with server key).
- Redact secrets from logs and SSE events.
- Bounded payloads, node timeouts, concurrency caps.
- Webhook path auth / signing where configured.
- Audit trail for mutate + run actions.
- AI tool allowlists and approval nodes (M10).

## Non-goals (MVP)
- Multi-tenant hard isolation beyond access checks.
- Exactly-once external side effects.
