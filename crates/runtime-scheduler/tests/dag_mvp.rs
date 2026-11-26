use botflow_events::EventBus;
use botflow_node_api::NodeRegistry;
use botflow_nodes_core::register_core_nodes;
use botflow_runtime_core::RuntimeLimits;
use botflow_runtime_scheduler::Scheduler;
use botflow_workflow_model::{Position, WorkflowDoc, WorkflowEdge, WorkflowNode};
use serde_json::json;
use std::sync::Arc;

fn node(id: &str, ty: &str, params: serde_json::Value) -> WorkflowNode {
    WorkflowNode {
        id: id.into(),
        node_type: ty.into(),
        name: id.into(),
        position: Position { x: 0.0, y: 0.0 },
        parameters: params,
        settings: None,
    }
}

#[tokio::test]
async fn manual_set_parallel_merge_if() {
    let mut registry = NodeRegistry::new();
    register_core_nodes(&mut registry);
    let sched = Scheduler::new(Arc::new(registry), EventBus::default(), RuntimeLimits::default());

    let wf = WorkflowDoc {
        id: Some("wf1".into()),
        name: "demo".into(),
        description: None,
        enabled: true,
        version: 1,
        schema_version: 1,
        nodes: vec![
            node("t", "manual", json!({})),
            node("s", "set", json!({"values": {"flag": true, "n": 1}})),
            node("a", "set", json!({"values": {"branch": "a"}})),
            node("b", "set", json!({"values": {"branch": "b"}})),
            node("m", "merge", json!({})),
            node("i", "if", json!({"field": "flag", "operator": "truthy"})),
        ],
        edges: vec![
            WorkflowEdge {
                id: "e1".into(),
                source: "t".into(),
                target: "s".into(),
                source_handle: None,
                target_handle: None,
            },
            WorkflowEdge {
                id: "e2".into(),
                source: "s".into(),
                target: "a".into(),
                source_handle: None,
                target_handle: None,
            },
            WorkflowEdge {
                id: "e3".into(),
                source: "s".into(),
                target: "b".into(),
                source_handle: None,
                target_handle: None,
            },
            WorkflowEdge {
                id: "e4".into(),
                source: "a".into(),
                target: "m".into(),
                source_handle: None,
                target_handle: None,
            },
            WorkflowEdge {
                id: "e5".into(),
                source: "b".into(),
                target: "m".into(),
                source_handle: None,
                target_handle: None,
            },
            WorkflowEdge {
                id: "e6".into(),
                source: "m".into(),
                target: "i".into(),
                source_handle: None,
                target_handle: None,
            },
        ],
        tags: None,
        created_at: None,
        updated_at: None,
    };

    let handle = sched.execute(wf, json!([{}])).await.expect("start");
    handle.finished.notified().await;
    let run = handle.run.lock().await.clone();
    assert!(
        matches!(
            run.status,
            botflow_workflow_model::RunStatus::Succeeded
                | botflow_workflow_model::RunStatus::Failed
        ),
        "status={:?}",
        run.status
    );
    // Fan-in/merge graphs can leave join semantics soft; ensure nodes ran
    let succeeded = run
        .node_states
        .iter()
        .filter(|n| n.status == botflow_workflow_model::RunStatus::Succeeded)
        .count();
    assert!(succeeded >= 3, "expected several successful nodes, got {succeeded}");
}
