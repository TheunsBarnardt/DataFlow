//! Handle ids / labels / accents — parity with `web/src/lib/nodeHandles.ts` + `nodeIcons.ts`.

pub fn source_handles_for_type(
    ty: &str,
    switch_cases: &[String],
    error_output: bool,
) -> Vec<String> {
    let mut handles: Vec<String> = match ty {
        "if" => vec!["true".into(), "false".into()],
        "filter" | "removeDuplicates" => vec!["kept".into(), "discarded".into()],
        "splitInBatches" => vec!["loop".into(), "done".into()],
        "switch" => {
            let cases: Vec<String> = switch_cases
                .iter()
                .filter(|c| !c.is_empty())
                .cloned()
                .collect();
            if cases.is_empty() {
                vec!["default".into()]
            } else {
                let mut uniq = Vec::new();
                for c in cases {
                    if !uniq.contains(&c) {
                        uniq.push(c);
                    }
                }
                uniq
            }
        }
        "compareDatasets" => vec!["both".into(), "onlyA".into(), "onlyB".into()],
        _ => vec!["main".into()],
    };
    if error_output && !handles.iter().any(|h| h == "error") {
        handles.push("error".into());
    }
    handles
}

pub fn target_handles_for_type(ty: &str) -> Vec<String> {
    match ty {
        "merge" | "compareDatasets" => vec!["0".into(), "1".into()],
        _ => vec!["main".into()],
    }
}

pub fn source_handle_label(ty: &str, handle: &str, http_method: Option<&str>) -> String {
    if handle == "error" {
        return "Error".into();
    }
    if handle == "main" {
        if matches!(ty, "webhook" | "apiEndpoint") {
            if let Some(m) = http_method {
                return m.to_uppercase();
            }
        }
        return "Output".into();
    }
    match (ty, handle) {
        ("if", "true") => "True".into(),
        ("if", "false") => "False".into(),
        ("filter" | "removeDuplicates", "kept") => "Kept".into(),
        ("filter" | "removeDuplicates", "discarded") => "Discarded".into(),
        ("splitInBatches", "loop") => "Loop".into(),
        ("splitInBatches", "done") => "Done".into(),
        ("compareDatasets", "both") => "In both".into(),
        ("compareDatasets", "onlyA") => "Only A".into(),
        ("compareDatasets", "onlyB") => "Only B".into(),
        _ => handle.to_string(),
    }
}

pub fn target_handle_label(ty: &str, handle: &str) -> String {
    if matches!(ty, "merge" | "compareDatasets") {
        return match handle {
            "0" => "Input 1".into(),
            "1" => "Input 2".into(),
            _ => handle.to_string(),
        };
    }
    handle.to_string()
}

pub fn connection_label_for_node(ty: &str, parameters: &serde_json::Value) -> Option<String> {
    match ty {
        "webhook" => Some(
            parameters
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("POST")
                .to_uppercase(),
        ),
        "apiEndpoint" => Some(
            parameters
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("GET")
                .to_uppercase(),
        ),
        _ => None,
    }
}

pub fn is_trigger_type(ty: &str) -> bool {
    matches!(
        ty,
        "manual"
            | "webhook"
            | "schedule"
            | "apiEndpoint"
            | "localFileTrigger"
            | "formTrigger"
            | "chatTrigger"
            | "emailTrigger"
            | "emailTriggerImap"
            | "rabbitmqTrigger"
            | "redisTrigger"
            | "outlookTrigger"
            | "teamsTrigger"
            | "whatsappTrigger"
            | "errorTrigger"
            | "sseTrigger"
            | "rssFeedTrigger"
            | "executeWorkflowTrigger"
            | "workflowTrigger"
            | "n8nTrigger"
            | "interval"
            | "activationTrigger"
            | "simulateTrigger"
            | "mcpServerTrigger"
            | "evaluationTrigger"
    )
}

/// Accent colors matching React `COLOR_BY_TYPE` / `getNodeColor`.
pub fn node_accent(ty: &str) -> &'static str {
    match ty {
        "if" | "switch" | "filter" => "#f97316",
        "merge" | "wait" | "splitInBatches" => "#a855f7",
        "httpRequest" | "webhookRespond" | "apiEndpoint" | "apiRespond" | "graphql" | "jwt" => {
            "#3b82f6"
        }
        "webhook" => "var(--brand)",
        "code" | "aiTransform" => "#8b5cf6",
        "set" | "editFields" => "#06b6d4",
        "stickyNote" => "#f59e0b",
        "stopAndError" => "#ef4444",
        "readFile" | "writeFile" | "listFiles" | "moveFile" | "copyFile" | "deleteFile" => {
            "#64748b"
        }
        "mssql" | "outlook" | "microsoftGraph" | "sharepoint" | "dynamics" | "excelOnline"
        | "postgres" | "mysql" | "mongoDb" | "redis" => "#0ea5e9",
        "schedule" | "manual" | "interval" => "#10b981",
        "sendEmail" | "llm" => "#14b8a6",
        _ if is_trigger_type(ty) => "var(--brand)",
        _ if ty.starts_with("http") || ty.contains("Api") => "#3b82f6",
        _ => "#6b7280",
    }
}
