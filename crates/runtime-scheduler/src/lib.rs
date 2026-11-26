use botflow_events::{EventBus, RunEvent, RunEventKind};
use botflow_node_api::{ExecutionContext, NodeError, NodeRegistry, NodeResult};
use botflow_runtime_core::{NodeOutputStore, RunHandle, RuntimeLimits};
use botflow_workflow_model::{RunStatus, WorkflowDoc};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info};

pub struct Scheduler {
    pub registry: Arc<NodeRegistry>,
    pub events: EventBus,
    pub limits: RuntimeLimits,
}

impl Scheduler {
    pub fn new(registry: Arc<NodeRegistry>, events: EventBus, limits: RuntimeLimits) -> Self {
        Self {
            registry,
            events,
            limits,
        }
    }

    pub async fn execute(
        &self,
        workflow: WorkflowDoc,
        input: Value,
    ) -> Result<RunHandle, NodeError> {
        self.execute_with_credentials(workflow, input, HashMap::new())
            .await
    }

    pub async fn execute_with_credentials(
        &self,
        workflow: WorkflowDoc,
        input: Value,
        credentials: HashMap<String, Value>,
    ) -> Result<RunHandle, NodeError> {
        let handle = RunHandle::new(&workflow, input.clone());
        let run_id = handle
            .run
            .lock()
            .await
            .id
            .clone()
            .unwrap_or_default();
        let workflow_id = workflow.id.clone().unwrap_or_default();
        let trace_id = handle.run.lock().await.trace_id.clone();

        {
            let mut run = handle.run.lock().await;
            run.status = RunStatus::Queued;
        }
        self.emit(
            &run_id,
            &workflow_id,
            &trace_id,
            RunEventKind::RunStatus,
            None,
            Some(RunStatus::Queued),
            None,
            None,
        );

        let registry = self.registry.clone();
        let events = self.events.clone();
        let limits = self.limits.clone();
        let cancel = handle.cancel.clone();
        let run_arc = handle.run.clone();
        let finished = handle.finished.clone();
        let wf = workflow.clone();

        tokio::spawn(async move {
            let result = run_graph(
                registry,
                events.clone(),
                limits,
                wf,
                input,
                run_arc.clone(),
                cancel,
                credentials,
            )
            .await;
            {
                let mut run = run_arc.lock().await;
                match result {
                    Ok(output) => {
                        run.status = RunStatus::Succeeded;
                        run.output = Some(output);
                    }
                    Err(NodeError::Cancelled) => {
                        run.status = RunStatus::Cancelled;
                        run.error = Some("cancelled".into());
                    }
                    Err(e) => {
                        run.status = RunStatus::Failed;
                        run.error = Some(e.to_string());
                    }
                }
                run.finished_at = Some(Utc::now());
                let status = run.status;
                let rid = run.id.clone().unwrap_or_default();
                let wid = run.workflow_id.clone();
                let tid = run.trace_id.clone();
                drop(run);
                events.publish(RunEvent {
                    run_id: rid,
                    workflow_id: wid,
                    trace_id: tid,
                    kind: RunEventKind::RunStatus,
                    at: Utc::now(),
                    node_id: None,
                    status: Some(status),
                    message: None,
                    data: None,
                });
            }
            finished.notify_waiters();
        });

        Ok(handle)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit(
        &self,
        run_id: &str,
        workflow_id: &str,
        trace_id: &str,
        kind: RunEventKind,
        node_id: Option<String>,
        status: Option<RunStatus>,
        message: Option<String>,
        data: Option<Value>,
    ) {
        self.events.publish(RunEvent {
            run_id: run_id.into(),
            workflow_id: workflow_id.into(),
            trace_id: trace_id.into(),
            kind,
            at: Utc::now(),
            node_id,
            status,
            message,
            data,
        });
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_graph(
    registry: Arc<NodeRegistry>,
    events: EventBus,
    limits: RuntimeLimits,
    workflow: WorkflowDoc,
    input: Value,
    run_arc: Arc<tokio::sync::Mutex<botflow_workflow_model::RunDoc>>,
    cancel: tokio_util::sync::CancellationToken,
    credentials: HashMap<String, Value>,
) -> Result<Value, NodeError> {
    {
        let mut run = run_arc.lock().await;
        run.status = RunStatus::Running;
    }

    let run_meta = {
        let run = run_arc.lock().await;
        (
            run.id.clone().unwrap_or_default(),
            run.workflow_id.clone(),
            run.trace_id.clone(),
        )
    };

    let node_map: HashMap<_, _> = workflow.nodes.iter().map(|n| (n.id.clone(), n.clone())).collect();
    let mut indegree: HashMap<String, usize> = workflow.nodes.iter().map(|n| (n.id.clone(), 0)).collect();
    let mut children: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for e in &workflow.edges {
        *indegree.entry(e.target.clone()).or_default() += 1;
        children
            .entry(e.source.clone())
            .or_default()
            .push((e.target.clone(), e.source_handle.clone()));
    }

    // Triggers with no inbound edges get the run input
    let mut ready: Vec<String> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| k.clone())
        .collect();

    let mut remaining: HashSet<String> = workflow.nodes.iter().map(|n| n.id.clone()).collect();
    let mut outputs = NodeOutputStore::default();
    let sem = Arc::new(Semaphore::new(limits.max_concurrent_nodes));
    let mut last_output = Value::Array(vec![input.clone()]);

    while !remaining.is_empty() {
        if cancel.is_cancelled() {
            return Err(NodeError::Cancelled);
        }
        if ready.is_empty() {
            return Err(NodeError::msg("deadlock: no ready nodes (unsupported cycle or join)"));
        }

        let batch: Vec<String> = ready.drain(..).collect();
        let mut futs = Vec::new();

        for node_id in batch {
            if !remaining.contains(&node_id) {
                continue;
            }
            let node = node_map
                .get(&node_id)
                .ok_or_else(|| NodeError::msg("missing node"))?
                .clone();
            if node.settings.as_ref().and_then(|s| s.disabled) == Some(true) {
                remaining.remove(&node_id);
                // treat as success with empty pass-through
                let incoming = gather_inputs(&workflow, &node_id, &outputs, &input);
                outputs.set(&node_id, None, incoming);
                unlock_children(&node_id, None, &children, &mut indegree, &mut ready, &remaining);
                continue;
            }

            let executor = registry.resolve_for_node(&node)?;
            let permit = sem.clone().acquire_owned().await.map_err(|e| NodeError::msg(e.to_string()))?;
            let cancel_n = cancel.clone();
            let timeout = Duration::from_millis(limits.default_node_timeout_ms);
            let incoming = gather_inputs(&workflow, &node_id, &outputs, &input);
            let run_id = run_meta.0.clone();
            let wid = run_meta.1.clone();
            let tid = run_meta.2.clone();
            let events_c = events.clone();
            let run_c = run_arc.clone();
            let creds = credentials.clone();

            futs.push(tokio::spawn(async move {
                let _permit = permit;
                set_node_status(&run_c, &node_id, RunStatus::Running, None, None, None).await;
                events_c.publish(RunEvent {
                    run_id: run_id.clone(),
                    workflow_id: wid.clone(),
                    trace_id: tid.clone(),
                    kind: RunEventKind::NodeStatus,
                    at: Utc::now(),
                    node_id: Some(node_id.clone()),
                    status: Some(RunStatus::Running),
                    message: None,
                    data: None,
                });

                let input_for_log = Value::Array(incoming.clone());
                let ctx = ExecutionContext {
                    run_id: run_id.clone(),
                    workflow_id: wid.clone(),
                    trace_id: tid.clone(),
                    node: node.clone(),
                    input_items: incoming,
                    cancel: cancel_n.child_token(),
                    timeout,
                    credentials: creds,
                    variables: HashMap::new(),
                };

                let exec_fut = executor.execute(ctx);
                let result = tokio::select! {
                    _ = cancel_n.cancelled() => Err(NodeError::Cancelled),
                    res = tokio::time::timeout(timeout, exec_fut) => {
                        match res {
                            Ok(inner) => inner,
                            Err(_) => Err(NodeError::Timeout),
                        }
                    }
                };

                match &result {
                    Ok(nr) => {
                        // UI expects flat item arrays (TS parity), not raw NodeResult.
                        let data = Some(ui_output_from_node_result(nr));
                        set_node_status(
                            &run_c,
                            &node_id,
                            RunStatus::Succeeded,
                            Some(input_for_log.clone()),
                            data.clone(),
                            None,
                        )
                        .await;
                        events_c.publish(RunEvent {
                            run_id,
                            workflow_id: wid,
                            trace_id: tid,
                            kind: RunEventKind::NodeStatus,
                            at: Utc::now(),
                            node_id: Some(node_id.clone()),
                            status: Some(RunStatus::Succeeded),
                            message: None,
                            data,
                        });
                    }
                    Err(e) => {
                        set_node_status(
                            &run_c,
                            &node_id,
                            RunStatus::Failed,
                            Some(input_for_log),
                            None,
                            Some(e.to_string()),
                        )
                        .await;
                        events_c.publish(RunEvent {
                            run_id,
                            workflow_id: wid,
                            trace_id: tid,
                            kind: RunEventKind::NodeStatus,
                            at: Utc::now(),
                            node_id: Some(node_id.clone()),
                            status: Some(RunStatus::Failed),
                            message: Some(e.to_string()),
                            data: None,
                        });
                    }
                }

                (node_id, result)
            }));
        }

        for fut in futs {
            let (node_id, result) = fut.await.map_err(|e| NodeError::msg(e.to_string()))?;
            remaining.remove(&node_id);
            match result {
                Ok(NodeResult { outputs: outs }) => {
                    if outs.is_empty() {
                        outputs.set(&node_id, None, vec![]);
                        unlock_children(&node_id, None, &children, &mut indegree, &mut ready, &remaining);
                    } else {
                        for o in &outs {
                            let handle = o.output_handle.as_deref();
                            outputs.set(&node_id, handle, o.items.clone());
                            last_output = Value::Array(o.items.clone());
                            unlock_children(
                                &node_id,
                                handle,
                                &children,
                                &mut indegree,
                                &mut ready,
                                &remaining,
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(node_id = %node_id, error = %e, "node failed");
                    return Err(e);
                }
            }
        }
    }

    info!(run_id = %run_meta.0, "run completed");
    Ok(last_output)
}

fn gather_inputs(
    workflow: &WorkflowDoc,
    node_id: &str,
    outputs: &NodeOutputStore,
    root_input: &Value,
) -> Vec<Value> {
    let parents: Vec<_> = workflow
        .edges
        .iter()
        .filter(|e| e.target == node_id)
        .collect();
    if parents.is_empty() {
        return match root_input {
            Value::Null => vec![json!({})],
            Value::Array(a) if a.is_empty() || a.iter().all(|v| v.is_null()) => {
                vec![json!({})]
            }
            Value::Array(a) => a.clone(),
            other => vec![other.clone()],
        };
    }
    let mut items = Vec::new();
    for e in parents {
        items.extend(outputs.get_items(&e.source));
    }
    if items.is_empty() {
        vec![json!({})]
    } else {
        items
    }
}

fn unlock_children(
    source: &str,
    produced_handle: Option<&str>,
    children: &HashMap<String, Vec<(String, Option<String>)>>,
    indegree: &mut HashMap<String, usize>,
    ready: &mut Vec<String>,
    remaining: &HashSet<String>,
) {
    if let Some(list) = children.get(source) {
        for (target, edge_handle) in list {
            // Branched outputs only follow edges with matching sourceHandle.
            let matches = match (produced_handle, edge_handle.as_deref()) {
                (Some(p), Some(h)) => p == h,
                (Some(_), None) => false,
                (None, _) => true,
            };
            if !matches {
                continue;
            }
            if let Some(d) = indegree.get_mut(target) {
                if *d > 0 {
                    *d -= 1;
                }
                if *d == 0 && remaining.contains(target) && !ready.contains(target) {
                    ready.push(target.clone());
                }
            }
        }
    }
}

fn ui_output_from_node_result(nr: &NodeResult) -> Value {
    if nr.outputs.len() == 1 {
        return Value::Array(nr.outputs[0].items.clone());
    }
    if nr.outputs.is_empty() {
        return Value::Array(vec![]);
    }
    let mut map = serde_json::Map::new();
    for out in &nr.outputs {
        let key = out
            .output_handle
            .clone()
            .unwrap_or_else(|| "main".into());
        map.insert(key, Value::Array(out.items.clone()));
    }
    // Keep a flat `items` alias from the first port for NDV table/schema.
    let first_items = nr
        .outputs
        .first()
        .map(|o| o.items.clone())
        .unwrap_or_default();
    json!({
        "items": first_items,
        "outputs": Value::Object(map),
    })
}

async fn set_node_status(
    run: &Arc<tokio::sync::Mutex<botflow_workflow_model::RunDoc>>,
    node_id: &str,
    status: RunStatus,
    input: Option<Value>,
    output: Option<Value>,
    error: Option<String>,
) {
    let mut run = run.lock().await;
    if let Some(ns) = run.node_states.iter_mut().find(|n| n.node_id == node_id) {
        ns.status = status;
        if matches!(status, RunStatus::Running) {
            ns.started_at = Some(Utc::now());
            ns.attempt += 1;
        }
        if matches!(
            status,
            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
        ) {
            ns.finished_at = Some(Utc::now());
        }
        if input.is_some() {
            ns.input = input;
        }
        if output.is_some() {
            ns.output = output;
        }
        if error.is_some() {
            ns.error = error;
        }
    }
}
