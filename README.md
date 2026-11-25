# DataFlow

Local TypeScript workflow automation: agent chat creates visible flows, schedules/webhooks run them, Cursor/Claude drive the same tools over MCP.

**Path:** `C:\Work\DataFlow`  
**Stack:** Node + Fastify + MongoDB + Vite/React/shadcn + React Flow  
**No Docker.** Internal use.

## Prerequisites

- Node.js 20+
- Local MongoDB on `mongodb://127.0.0.1:27017`

## Setup

```bash
cd C:\Work\DataFlow
npm install
npm run build -w shared
```

Copy `.env.example` to `.env` if needed.

## Run

```bash
npm run dev
```

- UI (Vite): http://127.0.0.1:5173  
- API: http://127.0.0.1:8787  

Production-style (API serves built UI):

```bash
npm run build
npm start
```

## MCP (Cursor / Claude)

```json
{
  "mcpServers": {
    "dataflow": {
      "command": "npm",
      "args": ["run", "mcp", "-w", "server"],
      "cwd": "C:\\Work\\DataFlow"
    }
  }
}
```

Tools: `list_workflows`, `get_workflow`, `create_workflow`, `update_workflow`, `run_workflow`, `get_run`, `create_webhook`, `create_schedule`, `list_node_types`.

## Observability

- **Insights** (`/api/insights`, UI tab): executions, failure rate, time saved, avg run time per workflow
- **Audit trail** (`/api/audit`, UI tab): create/edit/delete + run start/finish, tied to actor (`X-DataFlow-Actor` or `DATAFLOW_ACTOR`)
- **Step history**: open any run to inspect each node’s inputs, outputs, and errors
- **Export**: set `AUDIT_WEBHOOK_URL`, `METRICS_WEBHOOK_URL`, and/or `OTEL_ENDPOINT` to stream JSON events

## Reference forks

- `reference/n8n`
- `reference/OpenMausBot`

## Smoke

```bash
npm run smoke
```
