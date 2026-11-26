mod email;
mod files;
mod llm;
mod mssql;
mod postgres;

use botflow_node_api::NodeRegistry;
use std::sync::Arc;

pub use email::SendEmailNode;
pub use files::{ReadFileNode, WriteFileNode};
pub use llm::{ApprovalNode, LlmNode};
pub use mssql::MssqlNode;
pub use postgres::PostgresNode;

pub fn register_integration_nodes(registry: &mut NodeRegistry) {
    registry.register(Arc::new(MssqlNode));
    registry.register(Arc::new(PostgresNode));
    registry.register(Arc::new(SendEmailNode));
    registry.register(Arc::new(ReadFileNode));
    registry.register(Arc::new(WriteFileNode));
    registry.register(Arc::new(LlmNode::default()));
    registry.register(Arc::new(ApprovalNode));
}
