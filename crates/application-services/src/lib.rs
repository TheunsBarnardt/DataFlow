use anyhow::{anyhow, Result};
use botflow_events::EventBus;
use botflow_node_api::NodeRegistry;
use botflow_runtime_core::{RunHandle, RuntimeLimits};
use botflow_runtime_scheduler::Scheduler;
use botflow_security::{redact_value, SecretBox};
use botflow_storage::{self as storage, Db};
use botflow_workflow_model::{RunDoc, WorkflowDoc};
use botflow_workflow_validation::validate_workflow;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub registry: Arc<NodeRegistry>,
    pub events: EventBus,
    pub scheduler: Arc<Scheduler>,
    pub secrets: SecretBox,
    pub active_runs: Arc<RwLock<HashMap<String, RunHandle>>>,
}

impl AppState {
    pub fn new(db: Db, registry: NodeRegistry, secrets: SecretBox, limits: RuntimeLimits) -> Self {
        let registry = Arc::new(registry);
        let events = EventBus::default();
        let scheduler = Arc::new(Scheduler::new(registry.clone(), events.clone(), limits));
        Self {
            db,
            registry,
            events,
            scheduler,
            secrets,
            active_runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn list_workflows(&self) -> Result<Vec<WorkflowDoc>> {
        storage::list_workflows(&self.db).await
    }

    pub async fn get_workflow(&self, id: &str) -> Result<Option<WorkflowDoc>> {
        storage::get_workflow(&self.db, id).await
    }

    pub async fn create_workflow(&self, mut wf: WorkflowDoc) -> Result<WorkflowDoc> {
        // Studio "New workflow" starts empty; seed a manual trigger so validation + canvas work.
        if wf.nodes.is_empty() {
            wf.nodes.push(botflow_workflow_model::WorkflowNode {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Manual Trigger".into(),
                node_type: "manual".into(),
                position: botflow_workflow_model::Position { x: 120.0, y: 160.0 },
                parameters: serde_json::json!({}),
                settings: None,
            });
        }
        let issues = validate_workflow(&wf, Some(&self.registry));
        if !issues.ok() {
            return Err(anyhow!(
                "validation failed: {}",
                serde_json::to_string(&issues.issues).unwrap_or_default()
            ));
        }
        let saved = storage::insert_workflow(&self.db, wf).await?;
        let _ = storage::insert_audit(&self.db, "system", "workflow.create", &saved.name, "").await;
        self.sync_api_endpoint_registry(&saved).await;
        Ok(saved)
    }

    pub async fn update_workflow(&self, id: &str, wf: WorkflowDoc) -> Result<Option<WorkflowDoc>> {
        let issues = validate_workflow(&wf, Some(&self.registry));
        if !issues.ok() {
            return Err(anyhow!(
                "validation failed: {}",
                serde_json::to_string(&issues.issues).unwrap_or_default()
            ));
        }
        let saved = storage::update_workflow(&self.db, id, wf).await?;
        if let Some(ref wf) = saved {
            let _ = storage::insert_audit(&self.db, "system", "workflow.update", id, "").await;
            self.sync_api_endpoint_registry(wf).await;
        }
        Ok(saved)
    }

    pub async fn delete_workflow(&self, id: &str) -> Result<bool> {
        let _ = storage::delete_api_endpoints_for_workflow(&self.db, id).await;
        let ok = storage::delete_workflow(&self.db, id).await?;
        if ok {
            let _ = storage::insert_audit(&self.db, "system", "workflow.delete", id, "").await;
        }
        Ok(ok)
    }

    async fn sync_api_endpoint_registry(&self, wf: &WorkflowDoc) {
        let Some(wid) = wf.id.as_deref() else {
            return;
        };
        let _ = storage::sync_api_endpoints_from_workflow(
            &self.db,
            wid,
            wf.api_project_id.as_deref(),
            wf.kind.as_deref(),
            &wf.nodes,
        )
        .await;
    }

    pub async fn validate(&self, wf: &WorkflowDoc) -> botflow_workflow_validation::ValidationResult {
        validate_workflow(wf, Some(&self.registry))
    }

    pub async fn run_workflow(&self, id: &str, input: Value) -> Result<RunDoc> {
        let wf = storage::get_workflow(&self.db, id)
            .await?
            .ok_or_else(|| anyhow!("workflow not found"))?;
        let credentials = self.resolve_credentials_for_nodes(&wf.nodes).await;
        let handle = self
            .scheduler
            .execute_with_credentials(wf, input, credentials)
            .await
            .map_err(|e| anyhow!(e))?;
        let mut run = handle.run.lock().await.clone();
        run = storage::insert_run(&self.db, run).await?;
        {
            let mut guard = handle.run.lock().await;
            guard.id = run.id.clone();
        }
        let run_id = run.id.clone().unwrap_or_default();
        self.active_runs.write().await.insert(run_id.clone(), handle.clone());

        let db = self.db.clone();
        let active = self.active_runs.clone();
        let finished = handle.finished.clone();
        let run_arc = handle.run.clone();
        tokio::spawn(async move {
            finished.notified().await;
            let final_run = run_arc.lock().await.clone();
            let redacted = RunDoc {
                output: final_run.output.as_ref().map(redact_value),
                ..final_run
            };
            let _ = storage::update_run(&db, &redacted).await;
            active.write().await.remove(&run_id);
        });

        Ok(run)
    }

    /// Run a workflow and wait until it finishes (public `/apis/{slug}` dispatch).
    pub async fn run_workflow_and_wait(
        &self,
        id: &str,
        input: Value,
        timeout: Duration,
    ) -> Result<RunDoc> {
        let wf = storage::get_workflow(&self.db, id)
            .await?
            .ok_or_else(|| anyhow!("workflow not found"))?;
        let credentials = self.resolve_credentials_for_nodes(&wf.nodes).await;
        let handle = self
            .scheduler
            .execute_with_credentials(wf, input, credentials)
            .await
            .map_err(|e| anyhow!(e))?;
        let mut run = handle.run.lock().await.clone();
        run = storage::insert_run(&self.db, run).await?;
        {
            let mut guard = handle.run.lock().await;
            guard.id = run.id.clone();
        }
        let run_id = run.id.clone().unwrap_or_default();
        self.active_runs
            .write()
            .await
            .insert(run_id.clone(), handle.clone());

        let deadline = tokio::time::Instant::now() + timeout;
        let final_run = loop {
            {
                let guard = handle.run.lock().await;
                if guard.finished_at.is_some() {
                    break guard.clone();
                }
            }
            if tokio::time::Instant::now() >= deadline {
                handle.request_cancel();
                let _ = tokio::time::timeout(
                    Duration::from_secs(2),
                    handle.finished.notified(),
                )
                .await;
                let timed_out = handle.run.lock().await.clone();
                self.active_runs.write().await.remove(&run_id);
                let redacted = RunDoc {
                    output: timed_out.output.as_ref().map(redact_value),
                    ..timed_out
                };
                let _ = storage::update_run(&self.db, &redacted).await;
                return Err(anyhow!(
                    "API workflow timed out after {}s",
                    timeout.as_secs()
                ));
            }
            tokio::select! {
                _ = handle.finished.notified() => {},
                _ = tokio::time::sleep(Duration::from_millis(25)) => {},
                _ = tokio::time::sleep_until(deadline) => {},
            }
        };

        let redacted = RunDoc {
            output: final_run.output.as_ref().map(redact_value),
            ..final_run.clone()
        };
        let _ = storage::update_run(&self.db, &redacted).await;
        self.active_runs.write().await.remove(&run_id);
        Ok(final_run)
    }

    /// Decrypt credential blobs referenced by node `credentialId` parameters.
    async fn resolve_credentials_for_nodes(
        &self,
        nodes: &[botflow_workflow_model::WorkflowNode],
    ) -> HashMap<String, Value> {
        let mut ids = Vec::new();
        for node in nodes {
            collect_credential_ids(&node.parameters, &mut ids);
        }
        ids.sort();
        ids.dedup();

        let mut out = HashMap::new();
        for id in ids {
            match storage::get_credential(&self.db, &id).await {
                Ok(Some(c)) => {
                    let data = decrypt_credential_data(&self.secrets, &c.data);
                    out.insert(id, data);
                }
                Ok(None) => {
                    tracing::warn!(credential_id = %id, "credential not found for run");
                }
                Err(e) => {
                    tracing::warn!(credential_id = %id, error = %e, "failed to load credential");
                }
            }
        }
        out
    }

    /// Execute a single node (UI “Execute step”) — parity with TS `/execute-node`.
    pub async fn execute_single_node(
        &self,
        workflow_id: &str,
        node_id: &str,
        input: Option<Value>,
        parameters: Option<Value>,
    ) -> Result<Value> {
        use botflow_node_api::ExecutionContext;
        use std::time::Duration;
        use tokio_util::sync::CancellationToken;

        let wf = storage::get_workflow(&self.db, workflow_id)
            .await?
            .ok_or_else(|| anyhow!("workflow not found"))?;
        let mut node = wf
            .nodes
            .into_iter()
            .find(|n| n.id == node_id)
            .ok_or_else(|| anyhow!("node not found"))?;
        if let Some(params) = parameters {
            node.parameters = params;
        }
        let credentials = self.resolve_credentials_for_nodes(std::slice::from_ref(&node)).await;
        let executor = self
            .registry
            .resolve_for_node(&node)
            .map_err(|e| anyhow!(e))?;
        let input_items = match input {
            None | Some(Value::Null) => {
                if node.node_type == "apiEndpoint" {
                    api_endpoint_request_sample(&node.parameters)
                } else {
                    vec![json!({})]
                }
            }
            Some(Value::Array(arr)) if arr.is_empty() || arr.iter().all(|v| v.is_null()) => {
                if node.node_type == "apiEndpoint" {
                    api_endpoint_request_sample(&node.parameters)
                } else {
                    vec![json!({})]
                }
            }
            Some(Value::Array(arr)) => arr,
            Some(v) => vec![v],
        };
        let ctx = ExecutionContext {
            run_id: format!("step-{}", uuid::Uuid::new_v4()),
            workflow_id: workflow_id.to_string(),
            trace_id: uuid::Uuid::new_v4().to_string(),
            node,
            input_items,
            cancel: CancellationToken::new(),
            timeout: Duration::from_secs(60),
            credentials,
            variables: Default::default(),
        };
        match executor.execute(ctx).await {
            Ok(result) => {
                let output = if result.outputs.len() == 1 {
                    json!(result.outputs[0].items.clone())
                } else {
                    let mut map = serde_json::Map::new();
                    for out in result.outputs {
                        let key = out.output_handle.unwrap_or_else(|| "main".into());
                        map.insert(key, json!(out.items));
                    }
                    Value::Object(map)
                };
                Ok(json!({ "output": output }))
            }
            Err(e) => Ok(json!({ "output": null, "error": e.to_string() })),
        }
    }

    pub async fn cancel_run(&self, id: &str) -> Result<bool> {
        if let Some(h) = self.active_runs.read().await.get(id) {
            h.request_cancel();
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn get_run(&self, id: &str) -> Result<Option<RunDoc>> {
        if let Some(h) = self.active_runs.read().await.get(id) {
            return Ok(Some(h.run.lock().await.clone()));
        }
        storage::get_run(&self.db, id).await
    }

    pub async fn list_runs(&self, workflow_id: Option<&str>) -> Result<Vec<RunDoc>> {
        storage::list_runs(&self.db, workflow_id, 100).await
    }

    /// Broader run fetch for Insights (current + previous period).
    pub async fn list_runs_for_insights(&self) -> Result<Vec<RunDoc>> {
        storage::list_runs(&self.db, None, 5_000).await
    }
}

/// Request-shaped item for apiEndpoint Execute step when the client omits input.
fn api_endpoint_request_sample(parameters: &Value) -> Vec<Value> {
    let method = parameters
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");
    let path = parameters
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let query = sample_object_from_schema(parameters.get("requestQuerySchema"));
    let body = {
        let from_schema = sample_object_from_schema(parameters.get("requestBodySchema"));
        if from_schema.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            from_schema
        } else if matches!(
            method.to_ascii_uppercase().as_str(),
            "POST" | "PUT" | "PATCH"
        ) {
            json!({})
        } else {
            Value::Null
        }
    };
    vec![json!({
        "method": method,
        "path": path,
        "headers": {},
        "query": query,
        "body": body,
    })]
}

fn sample_object_from_schema(schema: Option<&Value>) -> Value {
    let Some(schema) = schema.filter(|s| s.is_object()) else {
        return json!({});
    };
    if schema.get("$ref").is_some() {
        return json!({});
    }
    let Some(props) = schema.get("properties").and_then(|v| v.as_object()) else {
        return json!({});
    };
    let mut out = serde_json::Map::new();
    for (k, prop) in props {
        if let Some(d) = prop.get("default").or_else(|| prop.get("example")) {
            out.insert(k.clone(), d.clone());
            continue;
        }
        let sample = match prop.get("type").and_then(|v| v.as_str()) {
            Some("integer") | Some("number") => json!(0),
            Some("boolean") => json!(false),
            Some("array") => json!([]),
            Some("object") => json!({}),
            _ => json!(""),
        };
        out.insert(k.clone(), sample);
    }
    Value::Object(out)
}

fn decrypt_credential_data(secrets: &SecretBox, blob: &str) -> Value {
    if let Ok(v) = secrets.decrypt_json(blob) {
        return v;
    }
    serde_json::from_str(blob).unwrap_or_else(|_| json!({}))
}

fn collect_credential_ids(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if k == "credentialId" || k.eq_ignore_ascii_case("credentials") {
                    if let Some(id) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        out.push(id.to_string());
                    }
                }
                collect_credential_ids(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_credential_ids(v, out);
            }
        }
        _ => {}
    }
}
