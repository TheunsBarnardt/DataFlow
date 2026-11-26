use anyhow::Result;
use botflow_application_services::AppState;
use botflow_integrations::register_integration_nodes;
use botflow_node_api::NodeRegistry;
use botflow_nodes_core::register_core_nodes;
use botflow_runtime_core::RuntimeLimits;
use botflow_security::SecretBox;
use botflow_storage::Db;
use chrono::{Duration, Utc};
use mongodb::bson::{doc, Bson};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let worker_id = std::env::var("WORKER_ID").unwrap_or_else(|_| Uuid::new_v4().to_string());
    let mongo_uri =
        std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://127.0.0.1:27017/botflow".into());
    let lease_secs: i64 = std::env::var("WORKER_LEASE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let db = Db::connect(&mongo_uri).await?;
    let mut registry = NodeRegistry::new();
    register_core_nodes(&mut registry);
    register_integration_nodes(&mut registry);
    let secrets = SecretBox::from_passphrase(
        &std::env::var("CREDENTIALS_KEY").unwrap_or_else(|_| "botflow-dev-credentials-key".into()),
    );
    let state = AppState::new(db.clone(), registry, secrets, RuntimeLimits::default());

    info!(%worker_id, "botflow-worker started (leases jobs from MongoDB)");

    loop {
        let now = Utc::now();
        let _ = db
            .workers
            .update_one(
                doc! { "workerId": &worker_id },
                doc! {
                    "$set": {
                        "workerId": &worker_id,
                        "lastHeartbeat": mongodb::bson::DateTime::from_millis(now.timestamp_millis()),
                    }
                },
            )
            .upsert(true)
            .await;

        let lease_until = now + Duration::seconds(lease_secs);
        let filter = doc! {
            "status": "queued",
            "availableAt": { "$lte": mongodb::bson::DateTime::from_millis(now.timestamp_millis()) },
        };

        let Some(job) = db.jobs.find_one(filter.clone()).await? else {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            continue;
        };

        let job_id = job.get("_id").cloned().unwrap_or(Bson::Null);
        let claimed = db
            .jobs
            .find_one_and_update(
                doc! {
                    "_id": job_id.clone(),
                    "status": "queued",
                },
                doc! {
                    "$set": {
                        "status": "leased",
                        "leaseOwner": &worker_id,
                        "leaseUntil": mongodb::bson::DateTime::from_millis(lease_until.timestamp_millis()),
                    },
                    "$inc": { "attempts": 1i32 }
                },
            )
            .await?;

        let Some(job) = claimed else {
            continue;
        };

        let workflow_id = job.get_str("workflowId").unwrap_or("").to_string();
        info!(%workflow_id, %worker_id, "leased job");
        match state
            .run_workflow(&workflow_id, serde_json::json!({ "from": "worker" }))
            .await
        {
            Ok(run) => {
                let _ = db
                    .jobs
                    .update_one(
                        doc! { "_id": job_id },
                        doc! { "$set": {
                            "status": "done",
                            "resultRunId": run.id.unwrap_or_default(),
                        }},
                    )
                    .await;
            }
            Err(e) => {
                warn!(error = %e, "job failed; re-queue");
                let _ = db
                    .jobs
                    .update_one(
                        doc! { "_id": job_id },
                        doc! {
                            "$set": {
                                "status": "queued",
                                "leaseOwner": Bson::Null,
                                "leaseUntil": Bson::Null,
                                "availableAt": mongodb::bson::DateTime::from_millis(
                                    (Utc::now() + Duration::seconds(5)).timestamp_millis()
                                ),
                                "lastError": e.to_string(),
                            }
                        },
                    )
                    .await;
            }
        }
    }
}
