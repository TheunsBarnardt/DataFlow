mod api_extra;
mod catalog;
mod cm_com;
mod control_extra;
mod data;
mod dataverse;
mod files_extra;
mod http;
mod logic;
mod microsoft;
mod triggers;
mod triggers_extra;
mod wait;

use botflow_node_api::NodeRegistry;

pub use catalog::{register_all_nodes, ALL_NODE_SPECS};
pub use http::HttpRequestNode;
pub use logic::{IfNode, MergeNode, SetNode};
pub use triggers::{ManualTrigger, ScheduleTrigger, WebhookTrigger};
pub use wait::WaitNode;

/// Legacy helper — prefers [`register_all_nodes`].
pub fn register_core_nodes(registry: &mut NodeRegistry) {
    register_all_nodes(registry);
}
