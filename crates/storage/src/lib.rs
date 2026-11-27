use anyhow::{Context, Result};
use botflow_workflow_model::{RunDoc, WorkflowDoc};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, Bson, Document},
    options::IndexOptions,
    Client, Collection, IndexModel,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::info;

#[derive(Clone)]
pub struct Db {
    pub client: Client,
    pub workflows: Collection<Document>,
    pub runs: Collection<Document>,
    pub credentials: Collection<Document>,
    pub audit_events: Collection<Document>,
    pub jobs: Collection<Document>,
    pub workers: Collection<Document>,
}

impl Db {
    pub async fn connect(uri: &str) -> Result<Self> {
        let client = Client::with_uri_str(uri)
            .await
            .context("mongodb connect")?;
        let db = client.default_database().unwrap_or_else(|| client.database("botflow"));
        let this = Self {
            client: client.clone(),
            workflows: db.collection("workflows"),
            runs: db.collection("runs"),
            credentials: db.collection("credentials"),
            audit_events: db.collection("audit_events"),
            jobs: db.collection("jobs"),
            workers: db.collection("workers"),
        };
        this.ensure_indexes().await?;
        info!(db = %db.name(), "mongodb ready");
        Ok(this)
    }

    async fn ensure_indexes(&self) -> Result<()> {
        self.workflows
            .create_index(IndexModel::builder().keys(doc! { "name": 1 }).build())
            .await?;
        self.runs
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "workflowId": 1, "startedAt": -1 })
                    .build(),
            )
            .await?;
        self.runs
            .create_index(IndexModel::builder().keys(doc! { "startedAt": -1 }).build())
            .await?;
        self.credentials
            .create_index(IndexModel::builder().keys(doc! { "name": 1 }).build())
            .await?;
        self.jobs
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "status": 1, "availableAt": 1 })
                    .build(),
            )
            .await?;
        self.workers
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "lastHeartbeat": 1 })
                    .build(),
            )
            .await?;
        let unique = IndexOptions::builder().unique(true).build();
        let _ = self
            .workers
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "workerId": 1 })
                    .options(unique.clone())
                    .build(),
            )
            .await;
        // Align with TypeScript `server/src/db.ts` collection names + indexes.
        let api_projects = api_projects_coll(self);
        let api_endpoints = api_endpoints_coll(self);
        let _ = api_projects
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "slug": 1 })
                    .options(unique)
                    .build(),
            )
            .await;
        let _ = api_endpoints
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "apiProjectId": 1, "method": 1, "path": 1 })
                    .build(),
            )
            .await;
        let _ = api_endpoints
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "workflowId": 1 })
                    .build(),
            )
            .await;
        let _ = self
            .workflows
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "apiProjectId": 1, "kind": 1 })
                    .build(),
            )
            .await;
        let api_keys = instance_api_keys_coll(self);
        let _ = api_keys
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "keyHash": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await;
        let _ = api_keys
            .create_index(IndexModel::builder().keys(doc! { "createdAt": -1 }).build())
            .await;
        Ok(())
    }
}

fn to_doc<T: Serialize>(value: &T) -> Result<Document> {
    let bson = mongodb::bson::to_bson(value)?;
    match bson {
        Bson::Document(d) => Ok(d),
        other => anyhow::bail!("expected document, got {other:?}"),
    }
}

fn from_doc<T: DeserializeOwned>(doc: Document) -> Result<T> {
    Ok(mongodb::bson::from_document(doc)?)
}

fn bson_to_rfc3339(b: &Bson) -> Option<String> {
    match b {
        Bson::DateTime(dt) => {
            let millis = dt.timestamp_millis();
            DateTime::from_timestamp_millis(millis).map(|d| d.to_rfc3339())
        }
        Bson::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn normalize_dt_field(doc: &mut Document, key: &str) {
    if let Some(b) = doc.get(key).cloned() {
        if let Some(s) = bson_to_rfc3339(&b) {
            doc.insert(key, s);
        }
    }
}

/// ObjectId / Date quirks + TS field aliases → something Serde can load into `RunDoc`.
fn normalize_run_document(mut doc: Document) -> Document {
    if let Some(id) = doc.remove("_id") {
        let s = match id {
            Bson::ObjectId(oid) => oid.to_hex(),
            Bson::String(s) => s,
            other => other.to_string(),
        };
        doc.insert("_id", s);
    }
    if let Some(wid) = doc.get("workflowId").cloned() {
        if let Bson::ObjectId(oid) = wid {
            doc.insert("workflowId", oid.to_hex());
        }
    }
    normalize_dt_field(&mut doc, "startedAt");
    normalize_dt_field(&mut doc, "finishedAt");

    if let Some(Bson::Array(logs)) = doc.get_mut("logs") {
        for entry in logs.iter_mut() {
            if let Bson::Document(d) = entry {
                normalize_dt_field(d, "startedAt");
                normalize_dt_field(d, "finishedAt");
            }
        }
    }

    if !doc.contains_key("traceId") {
        doc.insert("traceId", "");
    }
    if !doc.contains_key("workflowVersion") {
        doc.insert("workflowVersion", 0i32);
    }
    doc
}

fn parse_run_doc(doc: Document) -> Result<RunDoc> {
    let raw = doc.clone();
    let normalized = normalize_run_document(doc);
    let mut run: RunDoc = from_doc(normalized).or_else(|e| {
        // Last-resort: map via serde_json after stringifying extended JSON
        let json = mongodb::bson::from_document::<serde_json::Value>(normalize_run_document(raw))?;
        serde_json::from_value(json).map_err(|e2| anyhow::anyhow!("run decode: {e}; {e2}"))
    })?;
    // Prefer explicit logs → node_states when node_states empty
    if run.node_states.is_empty() {
        // already aliased via serde; nothing else
    }
    // Normalize log entry field names if they came through as nodeId only
    for ns in &mut run.node_states {
        if ns.node_name.is_none() {
            ns.node_name = Some(ns.node_id.clone());
        }
    }
    Ok(run)
}

/// Match `_id` whether Mongo stored a real ObjectId or a 24-hex string
/// (reimport/`mongoApi` historically inserted `String(new ObjectId())`).
fn id_filter(id: &str) -> Document {
    if let Ok(oid) = ObjectId::parse_str(id) {
        doc! { "$or": [ { "_id": oid }, { "_id": id } ] }
    } else {
        doc! { "_id": id }
    }
}

fn normalize_workflow_document(mut doc: Document) -> Document {
    if let Some(id) = doc.remove("_id") {
        let s = match id {
            Bson::ObjectId(oid) => oid.to_hex(),
            Bson::String(s) => s,
            other => other.to_string(),
        };
        doc.insert("_id", s);
    }
    for key in ["apiProjectId", "ownerId", "createdBy", "updatedBy"] {
        if let Some(Bson::ObjectId(oid)) = doc.get(key).cloned() {
            doc.insert(key, oid.to_hex());
        }
    }
    normalize_dt_field(&mut doc, "createdAt");
    normalize_dt_field(&mut doc, "updatedAt");
    doc
}

fn parse_workflow_doc(doc: Document) -> Result<WorkflowDoc> {
    let normalized = normalize_workflow_document(doc);
    from_doc(normalized.clone()).or_else(|e| {
        let json: serde_json::Value = mongodb::bson::from_document(normalized)?;
        serde_json::from_value(json).map_err(|e2| anyhow::anyhow!("workflow decode: {e}; {e2}"))
    })
}

pub async fn list_workflows(db: &Db) -> Result<Vec<WorkflowDoc>> {
    let mut cursor = db.workflows.find(doc! {}).await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(parse_workflow_doc(doc)?);
    }
    Ok(out)
}

pub async fn get_workflow(db: &Db, id: &str) -> Result<Option<WorkflowDoc>> {
    let Some(doc) = db.workflows.find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    Ok(Some(parse_workflow_doc(doc)?))
}

pub async fn insert_workflow(db: &Db, mut wf: WorkflowDoc) -> Result<WorkflowDoc> {
    let now = Utc::now();
    wf.created_at = Some(now);
    wf.updated_at = Some(now);
    if wf.version == 0 {
        wf.version = 1;
    }
    let mut doc = to_doc(&wf)?;
    doc.remove("_id");
    let res = db.workflows.insert_one(doc).await?;
    wf.id = match res.inserted_id {
        Bson::ObjectId(oid) => Some(oid.to_hex()),
        Bson::String(s) => Some(s),
        other => Some(other.to_string()),
    };
    Ok(wf)
}

pub async fn update_workflow(db: &Db, id: &str, mut wf: WorkflowDoc) -> Result<Option<WorkflowDoc>> {
    wf.updated_at = Some(Utc::now());
    wf.version = wf.version.saturating_add(1);
    wf.id = Some(id.to_string());
    let mut doc = to_doc(&wf)?;
    doc.remove("_id");
    let res = db
        .workflows
        .find_one_and_replace(id_filter(id), doc)
        .await?;
    Ok(res.map(|_| wf))
}

pub async fn delete_workflow(db: &Db, id: &str) -> Result<bool> {
    let res = db.workflows.delete_one(id_filter(id)).await?;
    Ok(res.deleted_count > 0)
}

pub async fn insert_run(db: &Db, mut run: RunDoc) -> Result<RunDoc> {
    let mut doc = to_doc(&run)?;
    doc.remove("_id");
    // camelCase fields already from serde
    let res = db.runs.insert_one(doc).await?;
    run.id = match res.inserted_id {
        Bson::ObjectId(oid) => Some(oid.to_hex()),
        Bson::String(s) => Some(s),
        other => Some(other.to_string()),
    };
    Ok(run)
}

pub async fn update_run(db: &Db, run: &RunDoc) -> Result<()> {
    let id = run.id.as_deref().context("run id required")?;
    let mut doc = to_doc(run)?;
    doc.remove("_id");
    db.runs.replace_one(id_filter(id), doc).await?;
    Ok(())
}

pub async fn get_run(db: &Db, id: &str) -> Result<Option<RunDoc>> {
    let Some(doc) = db.runs.find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    Ok(Some(parse_run_doc(doc)?))
}

pub async fn list_runs(db: &Db, workflow_id: Option<&str>, limit: i64) -> Result<Vec<RunDoc>> {
    let filter = match workflow_id {
        Some(wid) => doc! { "workflowId": wid },
        None => doc! {},
    };
    let mut cursor = db
        .runs
        .find(filter)
        .sort(doc! { "startedAt": -1 })
        .limit(limit)
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        match parse_run_doc(doc) {
            Ok(run) => out.push(run),
            Err(e) => tracing::warn!(error = %e, "skipping unreadable run document"),
        }
    }
    Ok(out)
}

pub async fn insert_audit(db: &Db, actor: &str, action: &str, resource: &str, detail: &str) -> Result<()> {
    db.audit_events
        .insert_one(doc! {
            "at": mongodb::bson::DateTime::now(),
            "actor": actor,
            "action": action,
            "resource": resource,
            "detail": detail,
        })
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub available_at: chrono::DateTime<Utc>,
    pub lease_owner: Option<String>,
    pub lease_until: Option<chrono::DateTime<Utc>>,
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub cred_type: String,
    /// Encrypted blob (AES-GCM base64)
    pub data: String,
    #[serde(default)]
    pub created_at: Option<chrono::DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub path: String,
    pub workflow_id: String,
    #[serde(default)]
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub workflow_id: String,
    pub cron: String,
    pub enabled: bool,
    #[serde(default)]
    pub next_run_at: Option<chrono::DateTime<Utc>>,
}

fn normalize_credential_document(mut doc: Document) -> Document {
    if let Some(id) = doc.remove("_id") {
        let s = match id {
            Bson::ObjectId(oid) => oid.to_hex(),
            Bson::String(s) => s,
            other => other.to_string(),
        };
        doc.insert("_id", s);
    }
    // List never returns secrets. Coerce legacy plaintext maps (or missing data)
    // so `CredentialDoc.data: String` does not fail with "invalid type: map".
    match doc.remove("data") {
        Some(Bson::String(s)) => {
            doc.insert("data", s);
        }
        _ => {
            doc.insert("data", "***");
        }
    }
    // chrono::DateTime serde expects RFC3339 strings; BSON DateTime arrives as a map.
    for key in ["createdAt", "updatedAt"] {
        normalize_dt_field(&mut doc, key);
    }
    doc
}

pub async fn list_credentials(db: &Db) -> Result<Vec<CredentialDoc>> {
    let mut cursor = db.credentials.find(doc! {}).await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        let normalized = normalize_credential_document(doc);
        match from_doc::<CredentialDoc>(normalized) {
            Ok(mut c) => {
                // Never return raw secret material in list — strip data
                c.data = "***".into();
                out.push(c);
            }
            Err(e) => {
                tracing::warn!(error = %e, "skipping unreadable credential document");
            }
        }
    }
    Ok(out)
}

/// Load one credential including the encrypted `data` blob (server-side only).
pub async fn get_credential(db: &Db, id: &str) -> Result<Option<CredentialDoc>> {
    let Some(mut doc) = db.credentials.find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    if let Some(idv) = doc.remove("_id") {
        let s = match idv {
            Bson::ObjectId(oid) => oid.to_hex(),
            Bson::String(s) => s,
            other => other.to_string(),
        };
        doc.insert("_id", s);
    }
    for key in ["createdAt", "updatedAt"] {
        normalize_dt_field(&mut doc, key);
    }
    // Legacy plaintext maps → JSON string so callers can parse without decrypt.
    if matches!(doc.get("data"), Some(Bson::Document(_))) {
        if let Some(Bson::Document(d)) = doc.remove("data") {
            let json = mongodb::bson::from_document::<serde_json::Value>(d)
                .ok()
                .and_then(|v| serde_json::to_string(&v).ok())
                .unwrap_or_else(|| "{}".into());
            doc.insert("data", json);
        }
    } else if !matches!(doc.get("data"), Some(Bson::String(_))) {
        doc.insert("data", "{}");
    }
    Ok(Some(from_doc::<CredentialDoc>(doc)?))
}

pub async fn upsert_credential(db: &Db, mut cred: CredentialDoc) -> Result<CredentialDoc> {
    let now = Utc::now();
    cred.updated_at = Some(now);
    if cred.created_at.is_none() {
        cred.created_at = Some(now);
    }
    let mut doc = to_doc(&cred)?;
    doc.remove("_id");
    if let Some(id) = &cred.id {
        db.credentials
            .replace_one(id_filter(id), doc)
            .await?;
    } else {
        let res = db.credentials.insert_one(doc).await?;
        cred.id = match res.inserted_id {
            Bson::ObjectId(oid) => Some(oid.to_hex()),
            _ => None,
        };
    }
    Ok(cred)
}

pub async fn delete_credential(db: &Db, id: &str) -> Result<bool> {
    Ok(db.credentials.delete_one(id_filter(id)).await?.deleted_count > 0)
}

fn normalize_variable_document(mut doc: Document) -> Document {
    if let Some(id) = doc.remove("_id") {
        let s = match id {
            Bson::ObjectId(oid) => oid.to_hex(),
            Bson::String(s) => s,
            other => other.to_string(),
        };
        doc.insert("_id", s);
    }
    // TS / legacy docs sometimes store value as a bare string; leave as-is.
    // If `key` is missing, synthesize from _id so list never hard-fails.
    if !doc.contains_key("key") {
        if let Some(Bson::String(id)) = doc.get("_id") {
            doc.insert("key", id.clone());
        }
    }
    doc
}

pub async fn list_variables(db: &Db) -> Result<Vec<VariableDoc>> {
    let mut cursor = db
        .client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("variables")
        .find(doc! {})
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        let normalized = normalize_variable_document(doc);
        match from_doc::<VariableDoc>(normalized.clone()) {
            Ok(v) => out.push(v),
            Err(e) => {
                // Skip corrupt docs instead of failing the whole list (React dashboard
                // Promise.all depends on /api/variables succeeding).
                tracing::warn!(error = %e, "skipping unreadable variable document");
            }
        }
    }
    Ok(out)
}

pub async fn upsert_variable(db: &Db, mut var: VariableDoc) -> Result<VariableDoc> {
    let coll = db
        .client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("variables");
    let mut doc = to_doc(&var)?;
    doc.remove("_id");
    coll.update_one(doc! { "key": &var.key }, doc! { "$set": doc })
        .upsert(true)
        .await?;
    if var.id.is_none() {
        if let Some(found) = coll.find_one(doc! { "key": &var.key }).await? {
            if let Some(Bson::ObjectId(oid)) = found.get("_id") {
                var.id = Some(oid.to_hex());
            }
        }
    }
    Ok(var)
}

pub async fn delete_variable(db: &Db, id: &str) -> Result<bool> {
    let coll = db
        .client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("variables");
    Ok(coll.delete_one(id_filter(id)).await?.deleted_count > 0)
}

pub async fn list_webhooks(db: &Db) -> Result<Vec<WebhookDoc>> {
    let coll = db
        .client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("webhooks");
    let mut cursor = coll.find(doc! {}).await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        let mut w: WebhookDoc = from_doc(doc.clone())?;
        if w.id.is_none() {
            if let Some(Bson::ObjectId(oid)) = doc.get("_id") {
                w.id = Some(oid.to_hex());
            }
        }
        out.push(w);
    }
    Ok(out)
}

pub async fn upsert_webhook(db: &Db, mut wh: WebhookDoc) -> Result<WebhookDoc> {
    let coll = db
        .client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("webhooks");
    if wh.method.is_empty() {
        wh.method = "POST".into();
    }
    let mut doc = to_doc(&wh)?;
    doc.remove("_id");
    coll.update_one(doc! { "path": &wh.path }, doc! { "$set": doc })
        .upsert(true)
        .await?;
    Ok(wh)
}

pub async fn list_schedules(db: &Db) -> Result<Vec<ScheduleDoc>> {
    let coll = db
        .client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("schedules");
    let mut cursor = coll.find(doc! {}).await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        let mut s: ScheduleDoc = from_doc(doc.clone())?;
        if s.id.is_none() {
            if let Some(Bson::ObjectId(oid)) = doc.get("_id") {
                s.id = Some(oid.to_hex());
            }
        }
        out.push(s);
    }
    Ok(out)
}

pub async fn upsert_schedule(db: &Db, mut sched: ScheduleDoc) -> Result<ScheduleDoc> {
    let coll = db
        .client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("schedules");
    let mut doc = to_doc(&sched)?;
    doc.remove("_id");
    coll.update_one(doc! { "workflowId": &sched.workflow_id }, doc! { "$set": doc })
        .upsert(true)
        .await?;
    Ok(sched)
}

pub async fn delete_runs(db: &Db, workflow_id: Option<&str>) -> Result<u64> {
    let filter = match workflow_id {
        Some(wid) => doc! { "workflowId": wid },
        None => doc! {},
    };
    Ok(db.runs.delete_many(filter).await?.deleted_count)
}

pub async fn list_audit(db: &Db, limit: i64) -> Result<Vec<Document>> {
    let mut cursor = db
        .audit_events
        .find(doc! {})
        .sort(doc! { "at": -1 })
        .limit(limit)
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(doc);
    }
    Ok(out)
}

pub async fn enqueue_job(db: &Db, run_id: &str, workflow_id: &str) -> Result<JobDoc> {
    let job = JobDoc {
        id: None,
        run_id: run_id.into(),
        workflow_id: workflow_id.into(),
        status: "queued".into(),
        available_at: Utc::now(),
        lease_owner: None,
        lease_until: None,
        attempts: 0,
    };
    let mut doc = to_doc(&job)?;
    doc.remove("_id");
    let res = db.jobs.insert_one(doc).await?;
    let mut job = job;
    job.id = match res.inserted_id {
        Bson::ObjectId(oid) => Some(oid.to_hex()),
        _ => None,
    };
    Ok(job)
}

fn api_projects_coll(db: &Db) -> Collection<Document> {
    // Same names as TypeScript (`server/src/db.ts`) so seeded Wealth/etc. show up.
    db.client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("api_projects")
}

fn api_endpoints_coll(db: &Db) -> Collection<Document> {
    db.client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
        .collection::<Document>("api_endpoints")
}

fn normalize_json_document(mut doc: Document) -> Document {
    if let Some(id) = doc.remove("_id") {
        let s = match id {
            Bson::ObjectId(oid) => oid.to_hex(),
            Bson::String(s) => s,
            other => other.to_string(),
        };
        doc.insert("_id", s);
    }
    for key in [
        "apiProjectId",
        "workflowId",
        "createdBy",
        "updatedBy",
        "ownerId",
    ] {
        if let Some(Bson::ObjectId(oid)) = doc.get(key).cloned() {
            doc.insert(key, oid.to_hex());
        }
    }
    for key in ["createdAt", "updatedAt", "at", "startedAt", "finishedAt"] {
        normalize_dt_field(&mut doc, key);
    }
    doc
}

fn document_to_json(doc: Document) -> Result<serde_json::Value> {
    let normalized = normalize_json_document(doc);
    Ok(mongodb::bson::from_document(normalized)?)
}

pub async fn list_api_projects(db: &Db) -> Result<Vec<serde_json::Value>> {
    let mut cursor = api_projects_coll(db)
        .find(doc! {})
        .sort(doc! { "updatedAt": -1 })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(document_to_json(doc)?);
    }
    Ok(out)
}

pub async fn get_api_project(db: &Db, id: &str) -> Result<Option<serde_json::Value>> {
    let Some(doc) = api_projects_coll(db).find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    Ok(Some(document_to_json(doc)?))
}

pub async fn find_api_project_by_slug(db: &Db, slug: &str) -> Result<Option<serde_json::Value>> {
    let Some(doc) = api_projects_coll(db).find_one(doc! { "slug": slug }).await? else {
        return Ok(None);
    };
    Ok(Some(document_to_json(doc)?))
}

pub async fn insert_api_project(db: &Db, project: serde_json::Value) -> Result<serde_json::Value> {
    let mut doc: Document = mongodb::bson::to_document(&project)?;
    doc.remove("_id");
    let res = api_projects_coll(db).insert_one(doc.clone()).await?;
    let id = match res.inserted_id {
        Bson::ObjectId(oid) => oid.to_hex(),
        Bson::String(s) => s,
        other => other.to_string(),
    };
    doc.insert("_id", &id);
    document_to_json(doc)
}

pub async fn update_api_project(
    db: &Db,
    id: &str,
    set: serde_json::Value,
) -> Result<Option<serde_json::Value>> {
    let set_doc: Document = mongodb::bson::to_document(&set)?;
    let res = api_projects_coll(db)
        .update_one(id_filter(id), doc! { "$set": set_doc })
        .await?;
    if res.matched_count == 0 {
        return Ok(None);
    }
    get_api_project(db, id).await
}

pub async fn delete_api_project(db: &Db, id: &str) -> Result<bool> {
    Ok(api_projects_coll(db)
        .delete_one(id_filter(id))
        .await?
        .deleted_count
        > 0)
}

pub async fn list_api_endpoints_for_project(
    db: &Db,
    api_project_id: &str,
) -> Result<Vec<serde_json::Value>> {
    let mut cursor = api_endpoints_coll(db)
        .find(doc! { "apiProjectId": api_project_id })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(document_to_json(doc)?);
    }
    Ok(out)
}

pub async fn list_api_endpoint_workflows(
    db: &Db,
    api_project_id: &str,
) -> Result<Vec<WorkflowDoc>> {
    let mut cursor = db
        .workflows
        .find(doc! {
            "apiProjectId": api_project_id,
            "kind": "apiEndpoint",
        })
        .sort(doc! { "name": 1 })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(parse_workflow_doc(doc)?);
    }
    Ok(out)
}

pub async fn delete_api_endpoints_for_project(db: &Db, api_project_id: &str) -> Result<u64> {
    Ok(api_endpoints_coll(db)
        .delete_many(doc! { "apiProjectId": api_project_id })
        .await?
        .deleted_count)
}

pub async fn delete_api_endpoints_for_workflow(db: &Db, workflow_id: &str) -> Result<u64> {
    Ok(api_endpoints_coll(db)
        .delete_many(doc! { "workflowId": workflow_id })
        .await?
        .deleted_count)
}

/// Upsert registry rows from canvas `apiEndpoint` nodes (TS `syncApiEndpointsFromWorkflow`).
pub async fn sync_api_endpoints_from_workflow(
    db: &Db,
    workflow_id: &str,
    api_project_id: Option<&str>,
    kind: Option<&str>,
    nodes: &[botflow_workflow_model::WorkflowNode],
) -> Result<Vec<serde_json::Value>> {
    if kind != Some("apiEndpoint") || api_project_id.is_none() {
        delete_api_endpoints_for_workflow(db, workflow_id).await?;
        return Ok(vec![]);
    }
    let api_project_id = api_project_id.unwrap();
    let respond = nodes.iter().find(|n| n.node_type == "apiRespond");
    let mut keep = std::collections::HashSet::new();
    let mut result = Vec::new();

    for node in nodes.iter().filter(|n| n.node_type == "apiEndpoint") {
        let path = node
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .trim_matches('/')
            .to_string();
        if path.is_empty() {
            continue;
        }
        let method = node
            .parameters
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();
        let method = match method.as_str() {
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" => method,
            _ => "GET".into(),
        };
        keep.insert(format!("{method}:{path}"));

        let auth = node
            .parameters
            .get("auth")
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        let auth = match auth {
            "jwt" | "headerBearer" | "apiKey" => auth,
            _ => "none",
        };
        let summary = node
            .parameters
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or(node.name.as_str())
            .to_string();
        let tag = node
            .parameters
            .get("tag")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                path.split('/')
                    .next()
                    .unwrap_or("default")
                    .to_string()
            });
        let op_id = node
            .parameters
            .get("operationId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let cleaned: String = path
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect();
                format!("{method}_{cleaned}")
            });

        let response_schema = respond
            .and_then(|n| n.parameters.get("responseSchema"))
            .cloned()
            .or_else(|| node.parameters.get("responseSchema").cloned())
            .unwrap_or(serde_json::json!({ "type": "object", "properties": {}, "required": [] }));
        let response_status = respond
            .and_then(|n| n.parameters.get("statusCode"))
            .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .or_else(|| {
                node.parameters
                    .get("responseStatus")
                    .and_then(|v| v.as_u64())
            })
            .unwrap_or(200) as i32;

        let mut set = doc! {
            "apiProjectId": api_project_id,
            "workflowId": workflow_id,
            "nodeId": &node.id,
            "path": &path,
            "method": &method,
            "summary": summary,
            "tag": tag,
            "auth": auth,
            "operationId": op_id,
            "responseStatus": response_status,
        };
        if let Ok(Bson::Document(d)) = mongodb::bson::to_bson(&response_schema) {
            set.insert("responseSchema", d);
        }
        for key in ["requestQuerySchema", "requestBodySchema"] {
            if let Some(schema) = node.parameters.get(key) {
                if let Ok(Bson::Document(d)) = mongodb::bson::to_bson(schema) {
                    set.insert(key, d);
                }
            }
        }

        let coll = api_endpoints_coll(db);
        if let Some(existing) = coll
            .find_one(doc! {
                "apiProjectId": api_project_id,
                "method": &method,
                "path": &path,
            })
            .await?
        {
            let Some(eid) = existing.get("_id").cloned() else {
                continue;
            };
            coll.update_one(doc! { "_id": eid.clone() }, doc! { "$set": set.clone() })
                .await?;
            let mut merged = existing;
            for (k, v) in set {
                merged.insert(k, v);
            }
            merged.insert("_id", eid);
            result.push(document_to_json(merged)?);
        } else {
            let mut doc = set;
            let oid = ObjectId::new();
            doc.insert("_id", oid);
            coll.insert_one(doc.clone()).await?;
            result.push(document_to_json(doc)?);
        }
    }

    let mut cursor = api_endpoints_coll(db)
        .find(doc! { "workflowId": workflow_id })
        .await?;
    while let Some(ep) = cursor.try_next().await? {
        let method = ep.get_str("method").unwrap_or("GET");
        let path = ep.get_str("path").unwrap_or("");
        let key = format!("{method}:{path}");
        if !keep.contains(&key) {
            if let Some(id) = ep.get("_id") {
                api_endpoints_coll(db)
                    .delete_one(doc! { "_id": id })
                    .await?;
            }
        }
    }

    Ok(result)
}

fn botflow_db(db: &Db) -> mongodb::Database {
    db.client
        .default_database()
        .unwrap_or_else(|| db.client.database("botflow"))
}

fn users_coll(db: &Db) -> Collection<Document> {
    botflow_db(db).collection::<Document>("user")
}

fn instance_settings_coll(db: &Db) -> Collection<Document> {
    botflow_db(db).collection::<Document>("instance_settings")
}

const USER_SECRET_FIELDS: &[&str] = &[
    "password",
    "passwordHash",
    "hashedPassword",
    "salt",
    "sessionToken",
    "refreshToken",
];

/// Local embedded-server password storage (not Better Auth bcrypt).
pub fn hash_local_password(password: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(password.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    format!("local-sha256:{hex}")
}

fn user_id_filter(id: &str) -> Document {
    doc! { "id": id }
}

fn strip_user_secrets(mut doc: Document) -> Document {
    for key in USER_SECRET_FIELDS {
        doc.remove(*key);
    }
    doc
}

fn user_document_to_json(doc: Document) -> Result<serde_json::Value> {
    let doc = strip_user_secrets(doc);
    document_to_json(doc)
}

pub async fn count_users(db: &Db) -> Result<u64> {
    Ok(users_coll(db).count_documents(doc! {}).await?)
}

pub async fn list_users(db: &Db) -> Result<Vec<serde_json::Value>> {
    let mut cursor = users_coll(db)
        .find(doc! {})
        .sort(doc! { "createdAt": -1 })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(user_document_to_json(doc)?);
    }
    Ok(out)
}

pub async fn find_user_by_id(db: &Db, id: &str) -> Result<Option<serde_json::Value>> {
    let Some(doc) = users_coll(db).find_one(user_id_filter(id)).await? else {
        return Ok(None);
    };
    Ok(Some(user_document_to_json(doc)?))
}

pub async fn find_user_by_identity(db: &Db, identity: &str) -> Result<Option<serde_json::Value>> {
    let filter = doc! {
      "$or": [
          { "id": identity },
          { "username": identity },
          { "email": identity },
      ]
  };
    let Some(doc) = users_coll(db).find_one(filter).await? else {
        return Ok(None);
    };
    Ok(Some(user_document_to_json(doc)?))
}

pub async fn update_user_by_id(
    db: &Db,
    id: &str,
    set: Document,
) -> Result<Option<serde_json::Value>> {
    if set.is_empty() {
        return find_user_by_id(db, id).await;
    }
    let res = users_coll(db)
        .update_one(user_id_filter(id), doc! { "$set": &set })
        .await?;
    if res.matched_count == 0 {
        return Ok(None);
    }
    find_user_by_id(db, id).await
}

pub async fn insert_user(db: &Db, mut doc: Document) -> Result<serde_json::Value> {
    doc.remove("_id");
    if !doc.contains_key("id") {
        doc.insert("id", uuid::Uuid::new_v4().to_string());
    }
    let now = Utc::now().to_rfc3339();
    if !doc.contains_key("createdAt") {
        doc.insert("createdAt", now.clone());
    }
    if !doc.contains_key("updatedAt") {
        doc.insert("updatedAt", now);
    }
    users_coll(db).insert_one(doc.clone()).await?;
    user_document_to_json(doc)
}

pub async fn upsert_local_user_profile(
    db: &Db,
    id: &str,
    name: Option<&str>,
    company_name: Option<&str>,
) -> Result<serde_json::Value> {
    let mut set = Document::new();
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        set.insert("name", n);
    }
    if let Some(c) = company_name {
        set.insert("companyName", c);
    }
    set.insert("updatedAt", Utc::now().to_rfc3339());
    let res = users_coll(db)
        .update_one(user_id_filter(id), doc! { "$set": &set })
        .await?;
    if res.matched_count == 0 {
        let doc = doc! {
            "id": id,
            "name": name.filter(|s| !s.is_empty()).unwrap_or("Local"),
            "email": "local@botflow",
            "username": "local",
            "displayUsername": "local",
            "role": "admin",
            "status": "active",
            "companyName": company_name.unwrap_or(""),
            "createdAt": Utc::now().to_rfc3339(),
            "updatedAt": Utc::now().to_rfc3339(),
        };
        return insert_user(db, doc).await;
    }
    find_user_by_id(db, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("user missing after upsert"))
}

const AUTH_SETTINGS_DOC_ID: &str = "authentication";

pub fn default_authentication_settings() -> serde_json::Value {
    serde_json::json!({
        "mfaEnforced": false,
        "sso": {
            "loginEnabled": false,
            "protocol": "saml",
            "saml": {
                "metadataUrl": "",
                "metadataXml": "",
                "metadataSource": "url",
                "entityId": "",
            },
            "oidc": {
                "discoveryEndpoint": "",
                "clientId": "",
                "clientSecret": "",
                "prompt": "select_account",
            },
        },
        "ldap": {
            "loginEnabled": false,
            "loginLabel": "LDAP Username or Email Address",
            "connectionUrl": "",
            "connectionPort": 389,
            "connectionSecurity": "none",
            "allowUnauthorizedCerts": false,
            "baseDn": "",
            "bindingAdminDn": "",
            "bindingAdminPassword": "",
            "userFilter": "",
            "loginIdAttribute": "uid",
            "emailAttribute": "mail",
            "firstNameAttribute": "givenName",
            "lastNameAttribute": "sn",
            "ldapIdAttribute": "uid",
            "synchronizationEnabled": false,
        },
    })
}

fn merge_auth_settings(stored: &serde_json::Value) -> serde_json::Value {
    let mut base = default_authentication_settings();
    if let Some(mfa) = stored.get("mfaEnforced").and_then(|v| v.as_bool()) {
        base["mfaEnforced"] = serde_json::json!(mfa);
    }
    if let Some(sso) = stored.get("sso").and_then(|v| v.as_object()) {
        if let Some(base_sso) = base.get_mut("sso").and_then(|v| v.as_object_mut()) {
            for (k, v) in sso {
                base_sso.insert(k.clone(), v.clone());
            }
            if let Some(saml) = sso.get("saml").and_then(|v| v.as_object()) {
                if let Some(base_saml) = base_sso.get_mut("saml").and_then(|v| v.as_object_mut()) {
                    for (k, v) in saml {
                        base_saml.insert(k.clone(), v.clone());
                    }
                }
            }
            if let Some(oidc) = sso.get("oidc").and_then(|v| v.as_object()) {
                if let Some(base_oidc) = base_sso.get_mut("oidc").and_then(|v| v.as_object_mut()) {
                    for (k, v) in oidc {
                        base_oidc.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }
    if let Some(ldap) = stored.get("ldap").and_then(|v| v.as_object()) {
        if let Some(base_ldap) = base.get_mut("ldap").and_then(|v| v.as_object_mut()) {
            for (k, v) in ldap {
                base_ldap.insert(k.clone(), v.clone());
            }
        }
    }
    base
}

pub fn public_authentication_settings(settings: &serde_json::Value) -> serde_json::Value {
    let mut out = settings.clone();
    let secret = out
        .pointer("/sso/oidc/clientSecret")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let redacted_secret = if secret.is_empty() { "" } else { "********" };
  if let Some(oidc) = out.get_mut("sso").and_then(|v| v.get_mut("oidc")) {
        if let Some(obj) = oidc.as_object_mut() {
            obj.insert(
                "clientSecret".into(),
                serde_json::Value::String(redacted_secret.into()),
            );
        }
    }
    let ldap_pw = out
        .pointer("/ldap/bindingAdminPassword")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let redacted_ldap = if ldap_pw.is_empty() { "" } else { "********" };
    if let Some(ldap) = out.get_mut("ldap").and_then(|v| v.as_object_mut()) {
        ldap.insert(
            "bindingAdminPassword".into(),
            serde_json::Value::String(redacted_ldap.into()),
        );
    }
    out
}

pub async fn get_authentication_settings(db: &Db) -> Result<serde_json::Value> {
    let doc = instance_settings_coll(db)
        .find_one(doc! { "_id": AUTH_SETTINGS_DOC_ID })
        .await?;
    let Some(doc) = doc else {
        return Ok(default_authentication_settings());
    };
    let stored = document_to_json(doc)?;
    Ok(merge_auth_settings(&stored))
}

pub async fn save_authentication_settings(
    db: &Db,
    next: serde_json::Value,
) -> Result<serde_json::Value> {
    let merged = merge_auth_settings(&next);
    let mut set = mongodb::bson::to_document(&merged)?;
    set.insert("updatedAt", Utc::now().to_rfc3339());
    instance_settings_coll(db)
        .update_one(
            doc! { "_id": AUTH_SETTINGS_DOC_ID },
            doc! { "$set": &set },
        )
        .upsert(true)
        .await?;
    Ok(merged)
}

// --- Instance API keys (DataFlow API settings) ---

pub const INSTANCE_API_KEY_PREFIX: &str = "bf_";

fn instance_api_keys_coll(db: &Db) -> Collection<Document> {
    botflow_db(db).collection::<Document>("instance_api_keys")
}

pub fn hash_instance_api_key(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(raw.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

fn public_api_key_json(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("keyHash");
        obj.remove("key");
        if let Some(id) = obj.remove("id").or_else(|| obj.get("_id").cloned()) {
            obj.insert("id".into(), id);
        }
        if !obj.contains_key("prefix") {
            obj.insert(
                "prefix".into(),
                serde_json::Value::String(INSTANCE_API_KEY_PREFIX.into()),
            );
        }
    }
    value
}

fn api_key_id_filter(id: &str) -> Document {
    if let Ok(oid) = ObjectId::parse_str(id) {
        doc! { "$or": [ { "_id": oid }, { "id": id } ] }
    } else {
        doc! { "$or": [ { "_id": id }, { "id": id } ] }
    }
}

/// Returns public key rows (no secret / hash).
pub async fn list_instance_api_keys(db: &Db) -> Result<Vec<serde_json::Value>> {
    let mut cursor = instance_api_keys_coll(db)
        .find(doc! {})
        .sort(doc! { "createdAt": -1 })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        let mut json = document_to_json(doc)?;
        if let Some(obj) = json.as_object_mut() {
            if let Some(id) = obj.get("_id").cloned() {
                obj.entry("id".to_string()).or_insert(id);
            }
        }
        out.push(public_api_key_json(json));
    }
    Ok(out)
}

pub struct CreatedInstanceApiKey {
    pub public: serde_json::Value,
    pub secret: String,
}

#[derive(Default)]
pub struct CreateInstanceApiKeyInput {
    pub name: String,
    pub user_id: Option<String>,
    pub expires_in_secs: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

pub async fn create_instance_api_key(
    db: &Db,
    input: CreateInstanceApiKeyInput,
) -> Result<CreatedInstanceApiKey> {
    let name = input.name.trim();
    if name.is_empty() {
        anyhow::bail!("name required");
    }
    let id = uuid::Uuid::new_v4().to_string();
    let random = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let secret = format!("{INSTANCE_API_KEY_PREFIX}{random}");
    let start: String = random.chars().take(8).collect();
    let key_hash = hash_instance_api_key(&secret);
    let now = Utc::now();
    let expires_at = input
        .expires_in_secs
        .filter(|&s| s > 0)
        .map(|secs| (now + chrono::Duration::seconds(secs)).to_rfc3339());
    let metadata = input.metadata.unwrap_or_else(|| serde_json::json!({}));

    let mut doc = doc! {
        "id": &id,
        "name": name,
        "prefix": INSTANCE_API_KEY_PREFIX,
        "start": &start,
        "keyHash": &key_hash,
        "enabled": true,
        "metadata": mongodb::bson::to_bson(&metadata)?,
        "createdAt": now.to_rfc3339(),
        "updatedAt": now.to_rfc3339(),
    };
    if let Some(uid) = input.user_id.filter(|s| !s.is_empty()) {
        doc.insert("userId", uid);
    }
    if let Some(exp) = expires_at {
        doc.insert("expiresAt", exp);
    }

    instance_api_keys_coll(db).insert_one(doc.clone()).await?;
    let mut public = document_to_json(doc)?;
    if let Some(obj) = public.as_object_mut() {
        obj.insert("id".into(), serde_json::Value::String(id));
    }
    Ok(CreatedInstanceApiKey {
        public: public_api_key_json(public),
        secret,
    })
}

pub async fn update_instance_api_key(
    db: &Db,
    id: &str,
    enabled: Option<bool>,
    name: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>> {
    let mut set = Document::new();
    if let Some(enabled) = enabled {
        set.insert("enabled", enabled);
    }
    if let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) {
        set.insert("name", name);
    }
    if let Some(metadata) = metadata {
        set.insert("metadata", mongodb::bson::to_bson(&metadata)?);
    }
    if set.is_empty() {
        anyhow::bail!("No values to update");
    }
    set.insert("updatedAt", Utc::now().to_rfc3339());
    let res = instance_api_keys_coll(db)
        .update_one(api_key_id_filter(id), doc! { "$set": &set })
        .await?;
    if res.matched_count == 0 {
        return Ok(None);
    }
    let Some(doc) = instance_api_keys_coll(db)
        .find_one(api_key_id_filter(id))
        .await?
    else {
        return Ok(None);
    };
    let mut json = document_to_json(doc)?;
    if let Some(obj) = json.as_object_mut() {
        if let Some(existing_id) = obj.get("_id").cloned() {
            obj.entry("id".to_string()).or_insert(existing_id);
        }
    }
    Ok(Some(public_api_key_json(json)))
}

pub async fn delete_instance_api_key(db: &Db, id: &str) -> Result<bool> {
    Ok(instance_api_keys_coll(db)
        .delete_one(api_key_id_filter(id))
        .await?
        .deleted_count
        > 0)
}

/// Verify a raw `X-Api-Key` value. Returns the public key document when valid.
pub async fn verify_instance_api_key(
    db: &Db,
    raw_key: &str,
) -> Result<Option<serde_json::Value>> {
    let raw = raw_key.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let hash = hash_instance_api_key(raw);
    let Some(doc) = instance_api_keys_coll(db)
        .find_one(doc! { "keyHash": hash })
        .await?
    else {
        return Ok(None);
    };
    let mut json = document_to_json(doc)?;
    let enabled = json
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !enabled {
        return Ok(None);
    }
    if let Some(exp) = json.get("expiresAt").and_then(|v| v.as_str()) {
        if let Ok(dt) = DateTime::parse_from_rfc3339(exp) {
            if dt < Utc::now() {
                return Ok(None);
            }
        }
    }
    if let Some(obj) = json.as_object_mut() {
        if let Some(id) = obj.get("_id").cloned() {
            obj.entry("id".to_string()).or_insert(id);
        }
    }
    Ok(Some(public_api_key_json(json)))
}

fn metadata_scoped_project_ids(meta: &serde_json::Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(arr) = meta.get("projectIds").and_then(|v| v.as_array()) {
        for v in arr {
            if let Some(s) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                ids.push(s.to_string());
            }
        }
    }
    // Legacy single-project field — treat as a one-element scope list.
    if let Some(s) = meta
        .get("projectId")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !ids.iter().any(|id| id == s) {
            ids.push(s.to_string());
        }
    }
    ids
}

/// Whether metadata allows the given method/path for an API project.
pub fn instance_api_key_allows(
    key: &serde_json::Value,
    project_id: &str,
    project_slug: &str,
    method: &str,
    path: &str,
) -> bool {
    let meta = key.get("metadata").unwrap_or(&serde_json::Value::Null);
    let scoped = metadata_scoped_project_ids(meta);
    if !scoped.is_empty()
        && !scoped
            .iter()
            .any(|id| id == project_id || id == project_slug)
    {
        return false;
    }
    if meta
        .get("readonly")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let m = method.to_uppercase();
        if m != "GET" && m != "HEAD" && m != "OPTIONS" {
            return false;
        }
    }
    if let Some(scopes) = meta.get("scopes").and_then(|v| v.as_array()) {
        if !scopes.is_empty() {
            let ok = scopes.iter().any(|s| {
                let sm = s
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*");
                let pat = s
                    .get("pathPattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*");
                let method_ok = sm == "*" || sm.eq_ignore_ascii_case(method);
                if !method_ok {
                    return false;
                }
                if pat == "*" {
                    return true;
                }
                let escaped = regex_lite_escape(pat).replace(r"\*", ".*");
                regex_is_match(&format!("^{escaped}$"), path)
            });
            if !ok {
                return false;
            }
        }
    }
    true
}

fn regex_lite_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            other => out.push(other),
        }
    }
    out
}

fn regex_is_match(pattern: &str, text: &str) -> bool {
    // Minimal glob-style matcher without pulling in the regex crate:
    // only supports `.*` sequences produced above.
    let parts: Vec<&str> = pattern.trim_start_matches('^').trim_end_matches('$').split(".*").collect();
    if parts.len() == 1 {
        return text.eq_ignore_ascii_case(parts[0]);
    }
    let lower = text.to_ascii_lowercase();
    let mut idx = 0usize;
    for (i, part) in parts.iter().enumerate() {
        let needle = part.to_ascii_lowercase();
        if needle.is_empty() {
            continue;
        }
        if i == 0 {
            if !lower.starts_with(&needle) {
                return false;
            }
            idx = needle.len();
            continue;
        }
        if i == parts.len() - 1 {
            return lower[idx..].ends_with(&needle) || lower[idx..].contains(&needle);
        }
        if let Some(pos) = lower[idx..].find(&needle) {
            idx += pos + needle.len();
        } else {
            return false;
        }
    }
    true
}
