# Core node specs (MVP)

| Type | Trigger | Inputs | Outputs | Key parameters |
|------|---------|--------|---------|----------------|
| manual | yes | — | main | — |
| webhook | yes | — | main | `path` |
| schedule | yes | — | main | `cron` / interval (trigger registration later) |
| httpRequest | no | main | main | `url`, `method`, `headers`, `body` |
| set | no | main | main | `values` / `json`, `keepOnlySet` |
| if | no | main | true, false | `field`, `operator`, `rightValue` / `value` |
| merge | no | main | main | — |
| wait | no | main | main | `amount` (ms) |

## Integrations

| Type | Notes |
|------|-------|
| mssql | `query`, `server`/`MSSQL_SERVER`, `database`; Tiberius pooled TCP |
| postgres | `query`, `connectionString`/`POSTGRES_URL` |
| sendEmail | `to`, `subject`, `text`, SMTP env |
| readFile / writeFile | `path` under `BOTFLOW_FILE_ROOT` |
| llm | `prompt`, `model`, allowlist + maxTokens budget |
| approval | `decision` or `APPROVAL_AUTO=1` |
