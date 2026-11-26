use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use serde_json::json;

pub struct PostgresNode;

#[async_trait]
impl NodeExecutor for PostgresNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "postgres".into(),
            display_name: "PostgreSQL".into(),
            category: "database".into(),
            description: "Execute SQL against PostgreSQL".into(),
            version: 1,
            is_trigger: false,
            inputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
            outputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let query = ctx
            .node
            .parameters
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("query is required".into()))?;
        let conn = ctx
            .node
            .parameters
            .get("connectionString")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("POSTGRES_URL").ok())
            .ok_or_else(|| NodeError::Config("connectionString or POSTGRES_URL required".into()))?;

        let (client, connection) = tokio_postgres::connect(&conn, tokio_postgres::NoTls)
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let rows = client
            .query(query, &[])
            .await
            .map_err(|e| NodeError::msg(e.to_string()))?;
        let mut out = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in row.columns().iter().enumerate() {
                let val: Option<String> = row.try_get(i).ok();
                obj.insert(
                    col.name().to_string(),
                    val.map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
            }
            out.push(serde_json::Value::Object(obj));
        }
        if out.is_empty() {
            out.push(json!({ "rowCount": 0 }));
        }
        Ok(NodeResult::single(out))
    }
}
