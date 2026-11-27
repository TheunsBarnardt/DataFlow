//! Generic platform auth: companies, roles, permissions, linked identities.
//! Sasfin AspNet/Dynamics shapes are one possible configuration — not hard-coded.

use anyhow::Result;
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    Collection,
};
use serde_json::{json, Value};

use crate::{botflow_db, document_to_json, Db};

fn companies_coll(db: &Db) -> Collection<Document> {
    botflow_db(db).collection::<Document>("companies")
}

fn roles_coll(db: &Db) -> Collection<Document> {
    botflow_db(db).collection::<Document>("roles")
}

fn linked_identities_coll(db: &Db) -> Collection<Document> {
    botflow_db(db).collection::<Document>("linked_identities")
}

fn id_filter(id: &str) -> Document {
    doc! { "id": id }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn str_vec(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|i| i.as_str().map(str::trim).filter(|s| !s.is_empty()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Built-in permission catalog (extensible via custom role permissions).
pub fn default_permission_catalog() -> Value {
    json!([
        { "id": "settings.manage", "label": "Manage settings & users", "group": "studio" },
        { "id": "workflows.read", "label": "View workflows", "group": "studio" },
        { "id": "workflows.write", "label": "Create and edit workflows", "group": "studio" },
        { "id": "credentials.manage", "label": "Manage credentials", "group": "studio" },
        { "id": "execute", "label": "Execute workflows", "group": "studio" },
        { "id": "api.invoke", "label": "Call API projects", "group": "api" },
        { "id": "mcp.invoke", "label": "Use MCP tools", "group": "api" },
        { "id": "companies.manage", "label": "Manage companies", "group": "auth" },
        { "id": "roles.manage", "label": "Manage roles & permissions", "group": "auth" },
        { "id": "identities.manage", "label": "Manage linked identities", "group": "auth" },
        { "id": "keys.manage", "label": "Manage API keys", "group": "auth" },
        { "id": "bff.invoke", "label": "Call screen / BFF endpoints", "group": "api" }
    ])
}

fn default_role_docs() -> Vec<Document> {
    vec![
        doc! {
            "id": "admin",
            "name": "Admin",
            "description": "Full instance control, including users and settings.",
            "builtin": true,
            "permissions": [
                "settings.manage", "workflows.read", "workflows.write", "credentials.manage",
                "execute", "api.invoke", "mcp.invoke", "companies.manage", "roles.manage",
                "identities.manage", "keys.manage", "bff.invoke"
            ],
            "createdAt": now(),
            "updatedAt": now(),
        },
        doc! {
            "id": "editor",
            "name": "Editor",
            "description": "Build and run workflows; cannot manage users or roles.",
            "builtin": true,
            "permissions": [
                "workflows.read", "workflows.write", "credentials.manage",
                "execute", "api.invoke", "bff.invoke"
            ],
            "createdAt": now(),
            "updatedAt": now(),
        },
        doc! {
            "id": "viewer",
            "name": "Viewer",
            "description": "Read-only access to workflows, runs, and insights.",
            "builtin": true,
            "permissions": ["workflows.read"],
            "createdAt": now(),
            "updatedAt": now(),
        },
        doc! {
            "id": "api_consumer",
            "name": "API consumer",
            "description": "Call published API project endpoints with scoped keys.",
            "builtin": true,
            "permissions": ["api.invoke"],
            "createdAt": now(),
            "updatedAt": now(),
        },
        doc! {
            "id": "mcp_consumer",
            "name": "MCP consumer",
            "description": "Invoke instance MCP tools from external agents/clients.",
            "builtin": true,
            "permissions": ["mcp.invoke"],
            "createdAt": now(),
            "updatedAt": now(),
        },
        doc! {
            "id": "shell_user",
            "name": "Shell user",
            "description": "End-user of a dumb client shell (BFF / screen APIs).",
            "builtin": true,
            "permissions": ["bff.invoke", "api.invoke"],
            "createdAt": now(),
            "updatedAt": now(),
        },
    ]
}

pub async fn ensure_platform_auth_seeded(db: &Db) -> Result<()> {
    let roles = roles_coll(db);
    for role in default_role_docs() {
        let id = role.get_str("id").unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let existing = roles.find_one(id_filter(&id)).await?;
        if existing.is_none() {
            roles.insert_one(role).await?;
        }
    }
    Ok(())
}

pub async fn list_permissions_catalog() -> Value {
    default_permission_catalog()
}

// --- Companies ---

pub async fn list_companies(db: &Db) -> Result<Vec<Value>> {
    let mut cursor = companies_coll(db)
        .find(doc! {})
        .sort(doc! { "name": 1 })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(document_to_json(doc)?);
    }
    Ok(out)
}

pub async fn get_company(db: &Db, id: &str) -> Result<Option<Value>> {
    let Some(doc) = companies_coll(db).find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    Ok(Some(document_to_json(doc)?))
}

pub async fn create_company(db: &Db, name: &str, external_ref: Option<&str>) -> Result<Value> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("name required");
    }
    let id = uuid::Uuid::new_v4().to_string();
    let mut doc = doc! {
        "id": &id,
        "name": name,
        "status": "active",
        "createdAt": now(),
        "updatedAt": now(),
    };
    if let Some(ext) = external_ref.map(str::trim).filter(|s| !s.is_empty()) {
        doc.insert("externalRef", ext);
    }
    companies_coll(db).insert_one(doc.clone()).await?;
    document_to_json(doc)
}

pub async fn update_company(
    db: &Db,
    id: &str,
    name: Option<&str>,
    status: Option<&str>,
    external_ref: Option<&str>,
) -> Result<Option<Value>> {
    let mut set = Document::new();
    if let Some(n) = name.map(str::trim).filter(|s| !s.is_empty()) {
        set.insert("name", n);
    }
    if let Some(s) = status.map(str::trim).filter(|s| !s.is_empty()) {
        set.insert("status", s);
    }
    if let Some(ext) = external_ref {
        set.insert("externalRef", ext.trim());
    }
    if set.is_empty() {
        anyhow::bail!("No values to update");
    }
    set.insert("updatedAt", now());
    let res = companies_coll(db)
        .update_one(id_filter(id), doc! { "$set": &set })
        .await?;
    if res.matched_count == 0 {
        return Ok(None);
    }
    get_company(db, id).await
}

pub async fn delete_company(db: &Db, id: &str) -> Result<bool> {
    Ok(companies_coll(db)
        .delete_one(id_filter(id))
        .await?
        .deleted_count
        > 0)
}

// --- Roles ---

pub async fn list_roles(db: &Db) -> Result<Vec<Value>> {
    ensure_platform_auth_seeded(db).await?;
    let mut cursor = roles_coll(db)
        .find(doc! {})
        .sort(doc! { "name": 1 })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(document_to_json(doc)?);
    }
    Ok(out)
}

pub async fn get_role(db: &Db, id: &str) -> Result<Option<Value>> {
    ensure_platform_auth_seeded(db).await?;
    let Some(doc) = roles_coll(db).find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    Ok(Some(document_to_json(doc)?))
}

pub async fn create_role(
    db: &Db,
    id: &str,
    name: &str,
    description: &str,
    permissions: &[String],
    api_access: Option<&Value>,
) -> Result<Value> {
    let id = id.trim().to_lowercase().replace(' ', "_");
    let name = name.trim();
    if id.is_empty() || name.is_empty() {
        anyhow::bail!("id and name required");
    }
    if roles_coll(db).find_one(id_filter(&id)).await?.is_some() {
        anyhow::bail!("role already exists");
    }
    let perms: Vec<_> = permissions
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| mongodb::bson::Bson::String(s.to_string()))
        .collect();
    let mut doc = doc! {
        "id": &id,
        "name": name,
        "description": description.trim(),
        "builtin": false,
        "permissions": perms,
        "createdAt": now(),
        "updatedAt": now(),
    };
    if let Some(access) = api_access {
        doc.insert("apiAccess", mongodb::bson::to_bson(access)?);
    }
    roles_coll(db).insert_one(doc.clone()).await?;
    document_to_json(doc)
}

pub async fn update_role(
    db: &Db,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    permissions: Option<&[String]>,
    api_access: Option<&Value>,
) -> Result<Option<Value>> {
    let mut set = Document::new();
    if let Some(n) = name.map(str::trim).filter(|s| !s.is_empty()) {
        set.insert("name", n);
    }
    if let Some(d) = description {
        set.insert("description", d.trim());
    }
    if let Some(perms) = permissions {
        let arr: Vec<_> = perms
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| mongodb::bson::Bson::String(s.to_string()))
            .collect();
        set.insert("permissions", arr);
    }
    if let Some(access) = api_access {
        set.insert("apiAccess", mongodb::bson::to_bson(access)?);
    }
    if set.is_empty() {
        anyhow::bail!("No values to update");
    }
    set.insert("updatedAt", now());
    let res = roles_coll(db)
        .update_one(id_filter(id), doc! { "$set": &set })
        .await?;
    if res.matched_count == 0 {
        return Ok(None);
    }
    get_role(db, id).await
}

/// Resolve effective API call access: user `apiAccess` fully replaces role when present.
/// Missing/`null` user override → role `apiAccess` (or `{}` = unrestricted within `api.invoke`).
pub async fn effective_api_access(
    db: &Db,
    user: Option<&Value>,
    role_id: &str,
) -> Result<Value> {
    if let Some(u) = user {
        if let Some(access) = u.get("apiAccess") {
            if !access.is_null() {
                return Ok(access.clone());
            }
        }
    }
    if let Some(role) = get_role(db, role_id).await? {
        if let Some(access) = role.get("apiAccess") {
            if !access.is_null() {
                return Ok(access.clone());
            }
        }
    }
    Ok(json!({}))
}

/// True when the permission list grants API invoke (or wildcard).
pub fn permissions_include_api_invoke(permissions: &[String]) -> bool {
    permissions.iter().any(|p| p == "api.invoke" || p == "*")
}

pub async fn delete_role(db: &Db, id: &str) -> Result<bool> {
    let Some(doc) = roles_coll(db).find_one(id_filter(id)).await? else {
        return Ok(false);
    };
    if doc.get_bool("builtin").unwrap_or(false) {
        anyhow::bail!("cannot delete builtin role");
    }
    Ok(roles_coll(db)
        .delete_one(id_filter(id))
        .await?
        .deleted_count
        > 0)
}

pub async fn permissions_for_role(db: &Db, role_id: &str) -> Result<Vec<String>> {
    let Some(role) = get_role(db, role_id).await? else {
        return Ok(Vec::new());
    };
    Ok(str_vec(&role, "permissions"))
}

pub fn role_has_permission(role: &Value, permission: &str) -> bool {
    str_vec(role, "permissions")
        .iter()
        .any(|p| p == permission || p == "*")
}

// --- Linked identities ---

pub async fn list_linked_identities(db: &Db, principal_id: Option<&str>) -> Result<Vec<Value>> {
    let filter = match principal_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(pid) => doc! { "principalId": pid },
        None => doc! {},
    };
    let mut cursor = linked_identities_coll(db)
        .find(filter)
        .sort(doc! { "createdAt": -1 })
        .await?;
    let mut out = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        out.push(document_to_json(doc)?);
    }
    Ok(out)
}

pub async fn create_linked_identity(
    db: &Db,
    principal_id: &str,
    provider: &str,
    external_id: &str,
    label: Option<&str>,
    is_default: bool,
    is_primary: bool,
    metadata: Option<Value>,
) -> Result<Value> {
    let principal_id = principal_id.trim();
    let provider = provider.trim();
    let external_id = external_id.trim();
    if principal_id.is_empty() || provider.is_empty() || external_id.is_empty() {
        anyhow::bail!("principalId, provider, and externalId required");
    }
    if is_default {
        linked_identities_coll(db)
            .update_many(
                doc! { "principalId": principal_id, "provider": provider },
                doc! { "$set": { "isDefault": false } },
            )
            .await?;
    }
    if is_primary {
        linked_identities_coll(db)
            .update_many(
                doc! { "principalId": principal_id },
                doc! { "$set": { "isPrimary": false } },
            )
            .await?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    let mut doc = doc! {
        "id": &id,
        "principalId": principal_id,
        "provider": provider,
        "externalId": external_id,
        "label": label.unwrap_or("").trim(),
        "isDefault": is_default,
        "isPrimary": is_primary,
        "createdAt": now(),
        "updatedAt": now(),
    };
    if let Some(meta) = metadata {
        doc.insert("metadata", mongodb::bson::to_bson(&meta)?);
    }
    linked_identities_coll(db).insert_one(doc.clone()).await?;
    document_to_json(doc)
}

pub async fn update_linked_identity(
    db: &Db,
    id: &str,
    label: Option<&str>,
    is_default: Option<bool>,
    is_primary: Option<bool>,
) -> Result<Option<Value>> {
    let Some(existing) = linked_identities_coll(db).find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    let principal_id = existing.get_str("principalId").unwrap_or("").to_string();
    let provider = existing.get_str("provider").unwrap_or("").to_string();
    if is_default == Some(true) {
        linked_identities_coll(db)
            .update_many(
                doc! { "principalId": &principal_id, "provider": &provider },
                doc! { "$set": { "isDefault": false } },
            )
            .await?;
    }
    if is_primary == Some(true) {
        linked_identities_coll(db)
            .update_many(
                doc! { "principalId": &principal_id },
                doc! { "$set": { "isPrimary": false } },
            )
            .await?;
    }
    let mut set = Document::new();
    if let Some(l) = label {
        set.insert("label", l.trim());
    }
    if let Some(d) = is_default {
        set.insert("isDefault", d);
    }
    if let Some(p) = is_primary {
        set.insert("isPrimary", p);
    }
    if set.is_empty() {
        anyhow::bail!("No values to update");
    }
    set.insert("updatedAt", now());
    linked_identities_coll(db)
        .update_one(id_filter(id), doc! { "$set": &set })
        .await?;
    let Some(doc) = linked_identities_coll(db).find_one(id_filter(id)).await? else {
        return Ok(None);
    };
    Ok(Some(document_to_json(doc)?))
}

pub async fn delete_linked_identity(db: &Db, id: &str) -> Result<bool> {
    Ok(linked_identities_coll(db)
        .delete_one(id_filter(id))
        .await?
        .deleted_count
        > 0)
}

/// Build auth context injected into workflow runs for a user principal.
pub async fn build_auth_context(db: &Db, principal_id: &str, role_id: &str) -> Result<Value> {
    let permissions = permissions_for_role(db, role_id).await?;
    let links = list_linked_identities(db, Some(principal_id)).await?;
    let user = crate::find_user_by_id(db, principal_id).await?;
    let api_access = effective_api_access(db, user.as_ref(), role_id).await?;
    let company_ids = user
        .as_ref()
        .and_then(|u| u.get("companyIds"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let company_name = user
        .as_ref()
        .and_then(|u| u.get("companyName"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let company_id = user
        .as_ref()
        .and_then(|u| u.get("companyId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(json!({
        "sub": principal_id,
        "role": role_id,
        "permissions": permissions,
        "apiAccess": api_access,
        "companyId": company_id,
        "companyIds": company_ids,
        "companyName": company_name,
        "linkedIdentities": links,
    }))
}

/// Platform cutover checklist status for operators.
pub fn cutover_checklist() -> Value {
    json!({
        "phases": [
            { "id": "auth-model", "label": "Auth model & storage", "status": "ready" },
            { "id": "management-ui", "label": "Settings management UI", "status": "ready" },
            { "id": "gateway", "label": "Public API gateway scopes", "status": "ready" },
            { "id": "integrations-nodes", "label": "Integrations as nodes (CM.com)", "status": "ready" },
            { "id": "bff-shells", "label": "BFF / dumb client shells", "status": "ready" },
            { "id": "legacy-retire", "label": "Retire Wealth/Online/Dynamics as authorities", "status": "ready" }
        ],
        "guidance": "Point shell clients at BotFlow /apis/{slug} using GET /api/bff/contract. Keep business logic in workflows. Use linked identities for external CRM ids. CM.com and other vendors are nodes + credentials."
    })
}
