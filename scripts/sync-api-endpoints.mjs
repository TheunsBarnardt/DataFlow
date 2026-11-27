#!/usr/bin/env node
/** Backfill api_endpoints registry from existing apiEndpoint workflows. */
const API = process.env.BOTFLOW_API || "http://127.0.0.1:8788";

async function api(method, path, body) {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  const json = text ? JSON.parse(text) : null;
  if (!res.ok) throw new Error(`${method} ${path} → ${res.status}: ${text.slice(0, 300)}`);
  return json;
}

async function main() {
  const { projects } = await api("GET", "/api/api-projects");
  for (const p of projects || []) {
    const id = p._id || p.id;
    const detail = await api("GET", `/api/api-projects/${id}`);
    const wfs = detail.workflows || [];
    console.log(`${p.slug}: ${wfs.length} workflows, ${(detail.endpoints || []).length} endpoints → syncing…`);
    let n = 0;
    for (const w of wfs) {
      const wid = w._id || w.id;
      // PUT same doc triggers sync once create/update sync is deployed;
      // until then, force via a no-op update of updatedAt through workflows API.
      const full = await api("GET", `/api/workflows/${wid}`);
      const wf = full.workflow || full;
      await api("PUT", `/api/workflows/${wid}`, wf);
      n++;
      if (n % 50 === 0) console.log(`  …${n}/${wfs.length}`);
    }
    const after = await api("GET", `/api/api-projects/${id}`);
    console.log(
      `  done: ${(after.endpoints || []).length} endpoints registered`,
    );
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
