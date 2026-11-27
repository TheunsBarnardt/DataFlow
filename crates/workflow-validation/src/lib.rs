use botflow_node_api::NodeRegistry;
use botflow_workflow_model::WorkflowDoc;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("workflow invalid ({0} issues)")]
    Invalid(usize),
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn ok(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|i| matches!(i.severity, Severity::Error))
    }

    pub fn push_error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
            severity: Severity::Error,
        });
    }

    pub fn push_warning(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            path: path.into(),
            message: message.into(),
            severity: Severity::Warning,
        });
    }
}

pub fn validate_workflow(doc: &WorkflowDoc, registry: Option<&NodeRegistry>) -> ValidationResult {
    let mut result = ValidationResult::default();

    if doc.name.trim().is_empty() {
        result.push_error("name", "workflow name is required");
    }

    if doc.nodes.is_empty() {
        result.push_error("nodes", "workflow must contain at least one node");
    }

    let mut ids = HashSet::new();
    for (i, node) in doc.nodes.iter().enumerate() {
        let path = format!("nodes[{i}]");
        if node.id.trim().is_empty() {
            result.push_error(format!("{path}.id"), "node id is required");
        } else if !ids.insert(node.id.clone()) {
            result.push_error(format!("{path}.id"), format!("duplicate node id '{}'", node.id));
        }
        if node.node_type.trim().is_empty() {
            result.push_error(format!("{path}.type"), "node type is required");
        } else if let Some(reg) = registry {
            if reg.get(&node.node_type).is_none() {
                result.push_error(
                    format!("{path}.type"),
                    format!("unknown node type '{}'", node.node_type),
                );
            }
        }
        if node.settings.as_ref().and_then(|s| s.disabled) == Some(true) {
            result.push_warning(format!("{path}"), format!("node '{}' is disabled", node.name));
        }
    }

    let node_ids: HashSet<_> = doc.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut edge_ids = HashSet::new();
    for (i, edge) in doc.edges.iter().enumerate() {
        let path = format!("edges[{i}]");
        if edge.id.trim().is_empty() {
            result.push_error(format!("{path}.id"), "edge id is required");
        } else if !edge_ids.insert(edge.id.clone()) {
            result.push_error(format!("{path}.id"), format!("duplicate edge id '{}'", edge.id));
        }
        if !node_ids.contains(edge.source.as_str()) {
            result.push_error(format!("{path}.source"), format!("unknown source '{}'", edge.source));
        }
        if !node_ids.contains(edge.target.as_str()) {
            result.push_error(format!("{path}.target"), format!("unknown target '{}'", edge.target));
        }
        if edge.source == edge.target {
            result.push_error(format!("{path}"), "self-loop edges are not allowed");
        }
    }

    if let Some(cycle) = find_cycle(doc) {
        result.push_error(
            "edges",
            format!("cycle detected involving node '{}'", cycle),
        );
    }

    // Require at least one trigger-like node when registry knows triggers
    if let Some(reg) = registry {
        let has_trigger = doc.nodes.iter().any(|n| {
            reg.get(&n.node_type)
                .map(|e| e.metadata().is_trigger)
                .unwrap_or(false)
        });
        if !doc.nodes.is_empty() && !has_trigger {
            result.push_warning("nodes", "workflow has no trigger node");
        }
    }

    result
}

fn find_cycle(doc: &WorkflowDoc) -> Option<String> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &doc.edges {
        adj.entry(e.source.as_str()).or_default().push(e.target.as_str());
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    fn dfs<'a>(
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Option<&'a str> {
        if visited.contains(node) {
            return None;
        }
        if !visiting.insert(node) {
            return Some(node);
        }
        if let Some(nexts) = adj.get(node) {
            for n in nexts {
                if let Some(c) = dfs(n, adj, visiting, visited) {
                    return Some(c);
                }
            }
        }
        visiting.remove(node);
        visited.insert(node);
        None
    }

    for n in &doc.nodes {
        if let Some(c) = dfs(n.id.as_str(), &adj, &mut visiting, &mut visited) {
            return Some(c.to_string());
        }
    }
    None
}

pub fn topological_order(doc: &WorkflowDoc) -> Result<Vec<String>, ValidationError> {
    let mut indegree: HashMap<&str, usize> = doc.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &doc.edges {
        adj.entry(e.source.as_str()).or_default().push(e.target.as_str());
        *indegree.entry(e.target.as_str()).or_default() += 1;
    }
    let mut q: VecDeque<&str> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| *k)
        .collect();
    let mut order = Vec::new();
    while let Some(n) = q.pop_front() {
        order.push(n.to_string());
        if let Some(nexts) = adj.get(n) {
            for t in nexts {
                if let Some(d) = indegree.get_mut(t) {
                    *d -= 1;
                    if *d == 0 {
                        q.push_back(t);
                    }
                }
            }
        }
    }
    if order.len() != doc.nodes.len() {
        return Err(ValidationError::Invalid(1));
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use botflow_workflow_model::{Position, WorkflowEdge, WorkflowNode};
    use serde_json::json;

    fn node(id: &str, ty: &str) -> WorkflowNode {
        WorkflowNode {
            id: id.into(),
            node_type: ty.into(),
            name: id.into(),
            position: Position { x: 0.0, y: 0.0 },
            parameters: json!({}),
            settings: None,
        }
    }

    #[test]
    fn rejects_cycle() {
        let doc = WorkflowDoc {
            id: None,
            name: "c".into(),
            description: None,
            enabled: true,
            version: 1,
            schema_version: 1,
            nodes: vec![node("a", "set"), node("b", "set")],
            edges: vec![
                WorkflowEdge {
                    id: "e1".into(),
                    source: "a".into(),
                    target: "b".into(),
                    source_handle: None,
                    target_handle: None,
                },
                WorkflowEdge {
                    id: "e2".into(),
                    source: "b".into(),
                    target: "a".into(),
                    source_handle: None,
                    target_handle: None,
                },
            ],
            tags: None,
            time_saved_minutes: None,
            kind: None,
            api_project_id: None,
            created_by: None,
            updated_by: None,
            owner_id: None,
            visibility: None,
            created_at: None,
            updated_at: None,
        };
        let r = validate_workflow(&doc, None);
        assert!(!r.ok());
        assert!(r.issues.iter().any(|i| i.message.contains("cycle")));
    }

    #[test]
    fn accepts_linear() {
        let doc = WorkflowDoc {
            id: None,
            name: "ok".into(),
            description: None,
            enabled: true,
            version: 1,
            schema_version: 1,
            nodes: vec![node("a", "manual"), node("b", "set")],
            edges: vec![WorkflowEdge {
                id: "e1".into(),
                source: "a".into(),
                target: "b".into(),
                source_handle: None,
                target_handle: None,
            }],
            tags: None,
            time_saved_minutes: None,
            kind: None,
            api_project_id: None,
            created_by: None,
            updated_by: None,
            owner_id: None,
            visibility: None,
            created_at: None,
            updated_at: None,
        };
        assert!(validate_workflow(&doc, None).ok());
    }
}
