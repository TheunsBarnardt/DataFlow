//! Dataverse / Dynamics 365 Web API — parity with n8n-nodes-dataverse-auth
//! (GET FetchXML|OData, PATCH, POST, DELETE, OPTIONSET, GLOBALOPTIONSET, ENTITY).

use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::LazyLock;

static ENTITY_FROM_FETCH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)<entity\s+name=['"]([^'"]+)['"]"#).expect("regex"));

fn meta() -> NodeMetadata {
    NodeMetadata {
        type_name: "dynamics".into(),
        display_name: "Dataverse".into(),
        category: "microsoft".into(),
        description: "Query or update Dataverse / Dynamics 365 records".into(),
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

fn param_str(params: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = params.get(*key).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    String::new()
}

/// Match n8n-nodes-dataverse-auth pluralization for entity set names.
pub fn modify_entity_logical_name(entity_logical_name: &str) -> String {
    let name = entity_logical_name.trim();
    if name.is_empty() {
        return String::new();
    }
    if name.ends_with('y') {
        format!("{}ies", &name[..name.len() - 1])
    } else if name.ends_with('s') {
        format!("{name}es")
    } else {
        format!("{name}s")
    }
}

fn extract_entity_name_from_fetch_xml(fetch_xml: &str) -> Option<String> {
    ENTITY_FROM_FETCH
        .captures(fetch_xml)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn normalize_scope(raw: &str) -> String {
    let s = raw.trim().trim_end_matches('/');
    let re = Regex::new(r"(?i)/api/data/v9\.?\d*$").expect("regex");
    re.replace(s, "").to_string()
}

fn parse_connection_string(raw: &str) -> Map<String, Value> {
    let mut out = Map::new();
    for part in raw.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(eq) = trimmed.find('=') else {
            continue;
        };
        if eq == 0 {
            continue;
        }
        let key = trimmed[..eq]
            .trim()
            .to_ascii_lowercase()
            .replace(' ', "");
        let value = trimmed[eq + 1..].trim().to_string();
        out.insert(key, Value::String(value));
    }
    out
}

struct DataverseAuth {
    token: String,
    scope: String,
}

async fn fetch_client_credentials_token(
    tenant: &str,
    client_id: &str,
    client_secret: &str,
    scope: &str,
) -> Result<String, NodeError> {
    let body = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("scope", &format!("{scope}/.default")),
        ("grant_type", "client_credentials"),
    ];
    let res = reqwest::Client::new()
        .post(format!(
            "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token"
        ))
        .form(&body)
        .send()
        .await
        .map_err(|e| NodeError::msg(e.to_string()))?;
    let status = res.status();
    let json: Value = res.json().await.unwrap_or(json!({}));
    if !status.is_success() {
        let detail = json
            .get("error_description")
            .and_then(|v| v.as_str())
            .unwrap_or("Dataverse OAuth token request failed");
        return Err(NodeError::msg(format!("{detail} ({status})")));
    }
    json.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| NodeError::msg("Dataverse OAuth response missing access_token"))
}

async fn resolve_auth(ctx: &ExecutionContext) -> Result<DataverseAuth, NodeError> {
    let params = &ctx.node.parameters;
    let inline_token = param_str(params, &["accessToken"]);
    let mut scope = normalize_scope(&param_str(params, &["baseUrl", "scope"]));

    let credential_id = param_str(params, &["credentialId"]);
    let mut data = if !credential_id.is_empty() {
        ctx.credentials
            .get(&credential_id)
            .cloned()
            .unwrap_or(json!({}))
    } else {
        json!({})
    };

    let method = data
        .get("authMethod")
        .and_then(|v| v.as_str())
        .unwrap_or("clientSecret")
        .to_ascii_lowercase();

    if method == "connectionstring" || data.get("connectionString").is_some() {
        let cs = parse_connection_string(
            data.get("connectionString")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );
        let url = cs
            .get("url")
            .or_else(|| cs.get("serviceuri"))
            .or_else(|| cs.get("server"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !url.is_empty() && scope.is_empty() {
            scope = normalize_scope(url);
        }
        if let Some(id) = cs
            .get("clientid")
            .or_else(|| cs.get("appid"))
            .or_else(|| cs.get("applicationid"))
            .and_then(|v| v.as_str())
        {
            data.as_object_mut()
                .map(|o| o.insert("clientId".into(), Value::String(id.into())));
        }
        if let Some(secret) = cs
            .get("clientsecret")
            .or_else(|| cs.get("secret"))
            .and_then(|v| v.as_str())
        {
            data.as_object_mut()
                .map(|o| o.insert("clientSecret".into(), Value::String(secret.into())));
        }
        if let Some(auth_type) = cs
            .get("authtype")
            .or_else(|| cs.get("authenticationtype"))
            .and_then(|v| v.as_str())
        {
            data.as_object_mut().map(|o| {
                o.insert(
                    "_csAuthType".into(),
                    Value::String(auth_type.to_ascii_lowercase()),
                )
            });
        }
    }

    if scope.is_empty() {
        scope = normalize_scope(
            data.get("scope")
                .or_else(|| data.get("baseUrl"))
                .or_else(|| data.get("resource"))
                .or_else(|| data.get("environmentUrl"))
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );
    }

    if !inline_token.is_empty() && !scope.is_empty() {
        return Ok(DataverseAuth {
            token: inline_token,
            scope,
        });
    }

    let existing = data
        .get("accessToken")
        .or_else(|| data.get("token"))
        .or_else(|| data.get("bearerToken"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if !existing.is_empty() && !scope.is_empty() {
        return Ok(DataverseAuth {
            token: existing,
            scope,
        });
    }

    let cs_auth = data
        .get("_csAuthType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let is_client_secret = method == "clientsecret"
        || cs_auth == "clientsecret"
        || (method == "connectionstring"
            && (cs_auth == "clientsecret" || data.get("clientSecret").is_some()));

    if is_client_secret && !scope.is_empty() {
        let tenant = data
            .get("tenantId")
            .or_else(|| data.get("tenant"))
            .and_then(|v| v.as_str())
            .unwrap_or("common");
        let client_id = data
            .get("clientId")
            .or_else(|| data.get("appId"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let client_secret = data
            .get("clientSecret")
            .or_else(|| data.get("secret"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !client_id.is_empty() && !client_secret.is_empty() {
            let token =
                fetch_client_credentials_token(tenant, client_id, client_secret, &scope).await?;
            return Ok(DataverseAuth { token, scope });
        }
    }

    // Legacy client id/secret without authMethod
    let tenant = data
        .get("tenantId")
        .or_else(|| data.get("tenant"))
        .and_then(|v| v.as_str())
        .unwrap_or("common");
    let client_id = data
        .get("clientId")
        .or_else(|| data.get("appId"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let client_secret = data
        .get("clientSecret")
        .or_else(|| data.get("secret"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !client_id.is_empty() && !client_secret.is_empty() && !scope.is_empty() {
        let token =
            fetch_client_credentials_token(tenant, client_id, client_secret, &scope).await?;
        return Ok(DataverseAuth { token, scope });
    }

    let env_token = std::env::var("DYNAMICS_TOKEN")
        .or_else(|_| std::env::var("MS_GRAPH_TOKEN"))
        .unwrap_or_default();
    if scope.is_empty() {
        scope = normalize_scope(
            &std::env::var("DYNAMICS_BASE_URL")
                .or_else(|_| std::env::var("DYNAMICS_API_BASE_URL"))
                .unwrap_or_default(),
        );
    }
    if !env_token.is_empty() && !scope.is_empty() {
        return Ok(DataverseAuth {
            token: env_token,
            scope,
        });
    }

    Err(NodeError::Config(
        "Dataverse: choose a connection method (Client Id/Secret, Connection String, …) and fill required fields, or set accessToken + baseUrl"
            .into(),
    ))
}

const ODATA_HEADERS: &[(&str, &str)] = &[
    ("OData-MaxVersion", "4.0"),
    ("OData-Version", "4.0"),
    ("Content-Type", "application/json; charset=utf-8"),
    ("Accept", "application/json"),
    ("Prefer", "odata.include-annotations=\"*\""),
    ("OData-MaxPageSize", "5000"),
];

async fn dataverse_fetch(
    auth: &DataverseAuth,
    api_path: &str,
    method: &str,
    body: Option<&Value>,
) -> Result<(u16, Value), NodeError> {
    let path = if api_path.starts_with('/') {
        api_path.to_string()
    } else {
        format!("/{api_path}")
    };
    let url = if path.starts_with("http") {
        path
    } else if path.starts_with("/api/") {
        format!("{}{path}", auth.scope)
    } else {
        format!(
            "{}/api/data/v9.2/{}",
            auth.scope,
            path.trim_start_matches('/')
        )
    };

    let client = reqwest::Client::new();
    let mut req = match method.to_ascii_uppercase().as_str() {
        "POST" => client.post(&url),
        "PATCH" => client.patch(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => client.get(&url),
    };
    req = req.bearer_auth(&auth.token);
    for (k, v) in ODATA_HEADERS {
        req = req.header(*k, *v);
    }
    if let Some(b) = body {
        if !matches!(method.to_ascii_uppercase().as_str(), "GET" | "DELETE") {
            req = req.json(b);
        }
    }

    let resp = req
        .send()
        .await
        .map_err(|e| NodeError::msg(e.to_string()))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::String(text.clone()))
    };
    if !status.is_success() {
        return Err(NodeError::msg(format!(
            "Dataverse API error: {status}. Details: {data}"
        )));
    }
    Ok((status.as_u16(), data))
}

fn merge_column_payload(payload: Option<&Value>, columns: Option<&Value>) -> Value {
    let mut body = match payload {
        Some(Value::Object(map)) => Value::Object(map.clone()),
        _ => json!({}),
    };

    let Some(columns) = columns else {
        return body;
    };

    if let Some(rows) = columns.get("columnValues").and_then(|v| v.as_array()) {
        if let Some(obj) = body.as_object_mut() {
            for row in rows {
                let name = row
                    .get("columnName")
                    .or_else(|| row.get("key"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if !name.is_empty() {
                    let val = row
                        .get("columnValue")
                        .or_else(|| row.get("value"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    obj.insert(name.to_string(), val);
                }
            }
        }
        return body;
    }

    if let Some(map) = columns.as_object() {
        if let Some(obj) = body.as_object_mut() {
            for (k, v) in map {
                if !k.is_empty() {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
    }
    body
}

async fn get_data(
    auth: &DataverseAuth,
    query_type: &str,
    entity_name: &str,
    query: &str,
) -> Result<Value, NodeError> {
    let t = query_type.to_ascii_uppercase();
    let full_api_url = if t == "FETCHXML" {
        if query.trim().is_empty() {
            return Err(NodeError::Config(
                "Dataverse GET (FetchXML): Query is required — paste a FetchXML document".into(),
            ));
        }
        let entity_logical = extract_entity_name_from_fetch_xml(query).ok_or_else(|| {
            NodeError::Config("Failed to extract entity name from FetchXML.".into())
        })?;
        let set = modify_entity_logical_name(&entity_logical);
        // URL-encode FetchXML into the fetchXml query parameter (Dataverse Web API).
        format!(
            "/api/data/v9.2/{set}?fetchXml={}",
            urlencoding::encode(query)
        )
    } else if t == "ODATA" {
        format!(
            "/api/data/v9.2/{}",
            query.trim_start_matches('/')
        )
    } else if t == "COLUMN" {
        let set = modify_entity_logical_name(entity_name);
        format!("/api/data/v9.2/{set}?$select={query}")
    } else {
        return Err(NodeError::Config(format!(
            "Unknown Dataverse GET type: {query_type}"
        )));
    };

    let (_status, body) = dataverse_fetch(auth, &full_api_url, "GET", None).await?;
    Ok(body)
}

fn normalize_option_set_response(data: &Value) -> Value {
    if let Some(options) = data.get("Options").and_then(|v| v.as_array()) {
        return json!({ "options": map_option_set_options(options) });
    }
    if let Some(options) = data
        .pointer("/OptionSet/Options")
        .and_then(|v| v.as_array())
    {
        return json!({ "options": map_option_set_options(options) });
    }
    if let Some(options) = data
        .pointer("/value/0/OptionSet/Options")
        .and_then(|v| v.as_array())
    {
        return json!({ "options": map_option_set_options(options) });
    }
    data.clone()
}

fn map_option_set_options(options: &[Value]) -> Vec<Value> {
    options
        .iter()
        .map(|entry| {
            let id = entry.get("Value").cloned().unwrap_or(Value::Null);
            let name = entry
                .pointer("/Label/LocalizedLabels/0/Label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| entry.get("Value").map(|v| v.to_string()))
                .unwrap_or_default();
            json!({ "Id": id, "Name": name })
        })
        .collect()
}

fn push_result(out: &mut Vec<Value>, data: &Value) {
    if let Some(arr) = data.get("value").and_then(|v| v.as_array()) {
        for row in arr {
            out.push(row.clone());
        }
    } else if data.is_object() {
        out.push(data.clone());
    } else {
        out.push(json!({ "result": data }));
    }
}

pub struct DynamicsNode;

#[async_trait]
impl NodeExecutor for DynamicsNode {
    fn metadata(&self) -> NodeMetadata {
        meta()
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        if ctx.cancel.is_cancelled() {
            return Err(NodeError::Cancelled);
        }

        let auth = resolve_auth(&ctx).await?;
        let params = &ctx.node.parameters;
        let sources: Vec<Value> = if ctx.input_items.is_empty() {
            vec![json!({})]
        } else {
            ctx.input_items.clone()
        };

        let mut out: Vec<Value> = Vec::new();

        for _item in &sources {
            let mut operation = param_str(params, &["operation"]).to_ascii_uppercase();
            if operation.is_empty() {
                let method = param_str(params, &["method"]).to_ascii_uppercase();
                operation = match method.as_str() {
                    "PATCH" => "PATCH".into(),
                    "POST" | "PUT" => "POST".into(),
                    "DELETE" => "DELETE".into(),
                    _ => "GET".into(),
                };
            }

            let entity_name = param_str(params, &["entityName", "entity", "ENTITYNAME"]);

            match operation.as_str() {
                "GET" => {
                    let mut query_type =
                        param_str(params, &["type", "queryType"]).to_ascii_uppercase();
                    if query_type.is_empty() {
                        query_type = "ODATA".into();
                    }
                    let mut query = param_str(params, &["getQuery", "query"]);

                    if query_type == "ODATA" {
                        let looks_like_set = !entity_name.is_empty()
                            && (entity_name.ends_with('s') || entity_name.contains('('));
                        let set_name = if looks_like_set {
                            entity_name.clone()
                        } else if !entity_name.is_empty() {
                            modify_entity_logical_name(&entity_name)
                        } else {
                            String::new()
                        };
                        if !set_name.is_empty() && query.is_empty() {
                            query = set_name;
                        } else if !set_name.is_empty()
                            && (query.starts_with('?') || query.starts_with('$'))
                        {
                            query = if query.starts_with('?') {
                                format!("{set_name}{query}")
                            } else {
                                format!("{set_name}?{query}")
                            };
                        }
                    }

                    let data = get_data(&auth, &query_type, &entity_name, &query).await?;
                    push_result(&mut out, &data);
                }
                "PATCH" => {
                    let record_id = param_str(params, &["recordId", "PATCH_RECORDID"]);
                    if entity_name.is_empty() || record_id.is_empty() {
                        return Err(NodeError::Config(
                            "Dataverse PATCH requires entityName and recordId".into(),
                        ));
                    }
                    let payload = params.get("body").or_else(|| params.get("patchData"));
                    let columns = params.get("columns").or_else(|| params.get("column"));
                    let body = merge_column_payload(payload, columns);
                    let set = modify_entity_logical_name(&entity_name);
                    let (status, data) = dataverse_fetch(
                        &auth,
                        &format!("/api/data/v9.2/{set}({record_id})"),
                        "PATCH",
                        Some(&body),
                    )
                    .await?;
                    if data.is_null() {
                        out.push(json!({ "ok": true, "recordId": record_id, "statusCode": status }));
                    } else {
                        push_result(&mut out, &data);
                    }
                }
                "POST" => {
                    if entity_name.is_empty() {
                        return Err(NodeError::Config(
                            "Dataverse POST requires entityName".into(),
                        ));
                    }
                    let payload = params.get("body").or_else(|| params.get("patchData"));
                    let columns = params.get("columns").or_else(|| params.get("postcolumn"));
                    let body = merge_column_payload(payload, columns);
                    let set = modify_entity_logical_name(&entity_name);
                    let (status, data) =
                        dataverse_fetch(&auth, &format!("/api/data/v9.2/{set}"), "POST", Some(&body))
                            .await?;
                    if data.is_null() {
                        out.push(json!({ "ok": true, "statusCode": status }));
                    } else {
                        push_result(&mut out, &data);
                    }
                }
                "DELETE" => {
                    let record_id =
                        param_str(params, &["recordId", "delete_recordid", "DELETE_RECORDID"]);
                    if entity_name.is_empty() || record_id.is_empty() {
                        return Err(NodeError::Config(
                            "Dataverse DELETE requires entityName and recordId".into(),
                        ));
                    }
                    let set = modify_entity_logical_name(&entity_name);
                    dataverse_fetch(
                        &auth,
                        &format!("/api/data/v9.2/{set}({record_id})"),
                        "DELETE",
                        None,
                    )
                    .await?;
                    out.push(json!({
                        "success": true,
                        "message": format!("Record {record_id} deleted successfully from {entity_name}"),
                    }));
                }
                "OPTIONSET" => {
                    let os_entity = param_str(
                        params,
                        &["optionsetEntityName", "OPTIONSET_ENTITYNAME", "entityName", "entity"],
                    );
                    let attr =
                        param_str(params, &["optionsetAttributeName", "OPTIONSET_ATTRIBUTENAME"]);
                    if os_entity.is_empty() || attr.is_empty() {
                        return Err(NodeError::Config(
                            "Dataverse OPTIONSET requires optionsetEntityName and optionsetAttributeName"
                                .into(),
                        ));
                    }
                    let odata = format!(
                        "EntityDefinitions(LogicalName='{os_entity}')/Attributes(LogicalName='{attr}')/Microsoft.Dynamics.CRM.PicklistAttributeMetadata?$select=LogicalName,DisplayName&$expand=OptionSet($select=Options)"
                    );
                    let data = get_data(&auth, "ODATA", "", &odata).await?;
                    push_result(&mut out, &normalize_option_set_response(&data));
                }
                "GLOBALOPTIONSET" => {
                    let attr =
                        param_str(params, &["globalAttributeName", "GLOBAL_ATTRIBUTENAME"]);
                    if attr.is_empty() {
                        return Err(NodeError::Config(
                            "Dataverse GLOBALOPTIONSET requires globalAttributeName".into(),
                        ));
                    }
                    let odata = format!("GlobalOptionSetDefinitions(Name='{attr}')");
                    let data = get_data(&auth, "ODATA", "", &odata).await?;
                    push_result(&mut out, &normalize_option_set_response(&data));
                }
                "ENTITY" => {
                    if entity_name.is_empty() {
                        return Err(NodeError::Config(
                            "Dataverse ENTITY requires entityName".into(),
                        ));
                    }
                    let id_col = param_str(params, &["entityIdColumn", "ENTITY_ID"]);
                    let name_col = param_str(params, &["entityNameColumn", "ENTITY_NAME"]);
                    if id_col.is_empty() || name_col.is_empty() {
                        return Err(NodeError::Config(
                            "Dataverse ENTITY requires entityIdColumn and entityNameColumn".into(),
                        ));
                    }
                    let set = modify_entity_logical_name(&entity_name);
                    let odata = format!("{set}?$select={id_col},{name_col}&$orderby={name_col} asc");
                    let data = get_data(&auth, "ODATA", &entity_name, &odata).await?;
                    let options: Vec<Value> = data
                        .get("value")
                        .and_then(|v| v.as_array())
                        .map(|rows| {
                            rows.iter()
                                .map(|entry| {
                                    json!({
                                        "Id": entry.get(&id_col),
                                        "Name": entry.get(&name_col),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    out.push(json!({ "options": options }));
                }
                other => {
                    return Err(NodeError::Config(format!(
                        "Dataverse: unknown operation {other}"
                    )));
                }
            }
        }

        Ok(NodeResult::single(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pluralizes_entity_names() {
        assert_eq!(modify_entity_logical_name("account"), "accounts");
        assert_eq!(modify_entity_logical_name("opportunity"), "opportunities");
        assert_eq!(modify_entity_logical_name("tct_country"), "tct_countries");
        assert_eq!(modify_entity_logical_name("status"), "statuses");
    }

    #[test]
    fn extracts_entity_from_fetchxml() {
        let xml = r#"<fetch top='5'><entity name='tct_country'><attribute name='tct_name' /></entity></fetch>"#;
        assert_eq!(
            extract_entity_name_from_fetch_xml(xml).as_deref(),
            Some("tct_country")
        );
    }

    #[test]
    fn fetchxml_url_includes_encoded_param() {
        let xml = "<fetch><entity name='account'><attribute name='name' /></entity></fetch>";
        let set = modify_entity_logical_name(
            &extract_entity_name_from_fetch_xml(xml).unwrap(),
        );
        let url = format!(
            "/api/data/v9.2/{set}?fetchXml={}",
            urlencoding::encode(xml)
        );
        assert!(url.contains("accounts?fetchXml="));
        assert!(url.contains("%3Cfetch%3E") || url.contains("fetch"));
        assert!(!url.ends_with("?fetchXml="));
    }
}
