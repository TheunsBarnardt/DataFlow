"""Generate BotFlow catalog.ts files from parsed ASP.NET controller inventories."""
from __future__ import annotations

import json
import re
from pathlib import Path

ONLINE_JSON = Path(r"C:\Work\BotFlow\scripts\online-endpoints.json")
DYN_JSON = Path(r"C:\Work\BotFlow\scripts\dynamics-endpoints.json")
SCHEMAS_JSON = Path(r"C:\Work\BotFlow\scripts\request-schemas.json")
OUT_ONLINE = Path(r"C:\Work\BotFlow\server\src\apis\wealthOnline\catalog.ts")
OUT_DYN = Path(r"C:\Work\BotFlow\server\src\apis\dynamics\catalog.ts")

# Public Online auth/register endpoints (AllowAnonymous or documented public)
ONLINE_PUBLIC = {
    ("Account", "LoginAsync"),
    ("Account", "RefreshAsync"),
    ("Account", "ForgotUsernameWealthAsync"),
    ("Account", "ResendForgotUsernameWealthAsync"),
    ("Account", "ForgotPasswordWealthAsync"),
    ("Account", "GetResetPasswordTokenAsync"),
    ("Account", "ResetPasswordWealthAsync"),
    ("Account", "ResendOtpSmsAsync"),
    ("Register", "RegisterAsync"),
    ("Register", "UsernameExistAsync"),
    ("Register", "ConfirmEmailAsync"),
    ("Register", "ConfirmPhoneAsync"),
    ("Register", "SendConfirmSmsAsync"),
    ("Register", "ResendEmailConfirmationAsync"),
    ("Register", "GetUserStatusAsync"),
    ("Support", "GetWebsiteFaqByCategory"),
    ("Support", "GetTechnicalData"),
    ("Support", "GetConvertedPhoneNumber"),
    ("Market", "GetTickerAsync"),
    ("PageContent", "GetPage"),
    ("PageContent", "GetContent"),
}

DYNAMICS_ENTITY = {
    "Client": "contacts",
    "SasfinAccount": "tct_sasfinaccountses",
    "Annotation": "annotations",
    "BankingDetails": "tct_bankdetailses",
    "Incident": "incidents",
    "Lead": "leads",
    "Opportunity": "opportunities",
    "Document": "annotations",
    "ReferenceData": "tct_referencedatas",
    "SystemUser": "systemusers",
    "RiskAssessment": "tct_riskassessments",
    "Proposal": "opportunities",
    "Provider": "tct_providers",
    "ResidentCountry": "tct_residentcountries",
    "CommunicationDistribution": "tct_communicationdistributions",
    "Connection": "connections",
    "InvestmentStrategy": "tct_investmentstrategies",
    "Lookup": "tct_referencedatas",
    "Quotation": "tct_quotations",
}


def strip_constraint(seg: str) -> str:
    # {id:guid} -> {id}; {referenceType}s -> {referenceType}s
    return re.sub(
        r"\{([^}:]+)(?::[^}]+)?\}",
        lambda m: "{" + m.group(1) + "}",
        seg,
    )


def build_path(ep: dict) -> str:
    area = ep.get("area") or ""
    controller = ep["controller"]
    action = ep["action"]
    template = ep.get("routeTemplate") or "[controller]/[action]"
    path = (
        template.replace("[area]", area)
        .replace("[controller]", controller)
        .replace("[action]", action)
    )
    path = re.sub(r"/+", "/", path).strip("/")
    parts = [p for p in path.split("/") if p]
    path = "/".join(parts)

    # Method-level [Route("...")] (e.g. Document Upload) — append
    extra = ep.get("extraRoute")
    if extra:
        cleaned = "/".join(strip_constraint(s) for s in extra.strip("/").split("/"))
        if cleaned:
            path = f"{path}/{cleaned}"

    suffix = ep.get("routeSuffix")
    if suffix:
        # HttpGet("Bank") / HttpPost("{id}/Won") — append to controller route
        cleaned = "/".join(strip_constraint(s) for s in suffix.strip("/").split("/"))
        if cleaned:
            path = f"{path}/{cleaned}"
    return path


def param_schema(name: str, typ: str) -> str:
    t = typ.strip()
    # strip nullable ?
    t = t.rstrip("?")
    if t in ("Guid", "System.Guid"):
        return f'{name}: {{ type: "string", format: "uuid" }}'
    if t in ("int", "Int32", "long", "Int64", "decimal", "double", "float"):
        return f'{name}: {{ type: "integer" }}'
    if t in ("bool", "Boolean"):
        return f'{name}: {{ type: "boolean" }}'
    if t in ("DateTime", "DateTimeOffset"):
        return f'{name}: {{ type: "string", format: "date-time" }}'
    return f'{name}: {{ type: "string" }}'


def parse_query_props(params: str) -> tuple[list[str], list[str]]:
    """Return (property lines, required names) for primitive FromQuery-ish params."""
    if not params.strip():
        return [], []
    props: list[str] = []
    required: list[str] = []
    # skip CancellationToken, complex body types without FromQuery
    for part in params.split(","):
        part = part.strip()
        if not part:
            continue
        if "CancellationToken" in part:
            continue
        # [FromBody] ...
        if "[FromBody]" in part or "FromBody" in part:
            continue
        # [FromForm]
        if "[FromForm]" in part:
            continue
        # strip attributes
        part = re.sub(r"\[[^\]]*\]", "", part).strip()
        # type name
        m = re.match(
            r"(?:required\s+)?([\w\.<>\?]+)\s+(\w+)(?:\s*=\s*[^,]+)?$",
            part,
        )
        if not m:
            continue
        typ, name = m.group(1), m.group(2)
        # skip complex DTOs (capitalized multi-word-ish without primitives)
        base = typ.rstrip("?").split(".")[-1]
        primitives = {
            "string",
            "Guid",
            "int",
            "Int32",
            "long",
            "Int64",
            "bool",
            "Boolean",
            "decimal",
            "double",
            "float",
            "DateTime",
            "DateTimeOffset",
        }
        # enums / simple look like Identifier
        if base not in primitives and not base.endswith("Enum") and not base.startswith("SasSec"):
            # allow IdentificationTypeEnum etc
            if "Enum" not in base and base[0].isupper() and len(base) > 3:
                # likely DTO — skip for query
                continue
        props.append(param_schema(name, typ))
        if not typ.endswith("?") and " = " not in part and "=" not in part.split(name)[-1]:
            # required if non-nullable and no default — heuristic: Guid/string without ?
            if not typ.endswith("?"):
                required.append(name)
    return props, required


def has_body(method: str, params: str) -> bool:
    if method in ("GET", "HEAD", "DELETE"):
        # DELETE sometimes has body but rare
        if method == "DELETE":
            return "[FromBody]" in params or "FromBody" in params
        return False
    if "[FromBody]" in params or "FromBody" in params:
        return True
    # POST with a complex first param (DTO) and no FromQuery-only
    for part in params.split(","):
        part = re.sub(r"\[[^\]]*\]", "", part).strip()
        if not part or "CancellationToken" in part:
            continue
        m = re.match(r"([\w\.<>\?]+)\s+(\w+)", part)
        if not m:
            continue
        typ = m.group(1).rstrip("?").split(".")[-1]
        primitives = {
            "string",
            "Guid",
            "int",
            "Int32",
            "long",
            "Int64",
            "bool",
            "Boolean",
            "decimal",
            "double",
            "float",
            "DateTime",
            "DateTimeOffset",
        }
        if typ not in primitives and "Enum" not in typ:
            return True
    return False


def ts_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def schema_props_literal(props: dict) -> str:
    """Emit `{ key: { type: \"string\" }, ... }` for queryProps/bodyProps."""
    parts: list[str] = []
    for name, schema in props.items():
        inner = ", ".join(
            f"{k}: {json.dumps(v)}" for k, v in schema.items() if k in ("type", "format")
        )
        parts.append(f"{name}: {{ {inner} }}")
    return "{ " + ", ".join(parts) + " }"


def emit_endpoint(
    ep: dict,
    *,
    auth: str,
    extra: dict | None = None,
    extracted: dict | None = None,
) -> str:
    path_comment = build_path(ep)
    key = f'{ep["controller"]}_{ep["action"]}'
    ex = (extracted or {}).get(key) or {}
    props, required = parse_query_props(ep.get("params") or "")
    # Prefer flattened extracted query props when richer
    if ex.get("query") and (not props or len(ex["query"]) >= len(props)):
        q = ex["query"]
        props_obj = schema_props_literal(q)
        query_line = f"    requestQuerySchema: queryProps({props_obj}),"
    elif props:
        props_obj = ", ".join(props)
        if required:
            req = ", ".join(json.dumps(r) for r in required)
            query_line = f"    requestQuerySchema: queryProps({{ {props_obj} }}, [{req}]),"
        else:
            query_line = f"    requestQuerySchema: queryProps({{ {props_obj} }}),"
    else:
        query_line = None

    body_line = None
    if ex.get("body"):
        body_line = f"    requestBodySchema: queryProps({schema_props_literal(ex['body'])}),"
    elif has_body(ep["method"], ep.get("params") or ""):
        # Still mark body present but empty properties — reapply can fill later
        body_line = '    requestBodySchema: queryProps({}),'

    lines = [
        f'  ep({json.dumps(ep["controller"])}, {json.dumps(ep["action"])}, {json.dumps(ep["method"])}, {json.dumps(auth)}, {json.dumps(ep["action"])}, {{',
        f'    path: {json.dumps(path_comment)},',
    ]
    if ep.get("area"):
        lines.append(f'    area: {json.dumps(ep["area"])},')
    if query_line:
        lines.append(query_line)
    if body_line:
        lines.append(body_line)
    lines.append('    responseSchema: { type: "object" },')
    if extra:
        for k, v in extra.items():
            lines.append(f"    {k}: {json.dumps(v)},")
    lines.append("  }),")
    return "\n".join(lines)


def generate_online(endpoints: list[dict], extracted: dict) -> str:
    chunks = [
        'import type { ApiAuthMode, ApiHttpMethod, JsonSchema } from "@botflow/shared";',
        'import { queryProps } from "../wealth/schemas.js";',
        "",
        "export interface OnlineEndpointDef {",
        "  controller: string;",
        "  action: string;",
        "  method: ApiHttpMethod;",
        "  auth: ApiAuthMode;",
        "  summary: string;",
        "  /** Explicit route path (area/controller/action + suffixes). */",
        "  path: string;",
        "  area?: string;",
        '  impl?: "login" | "refresh" | "proxy" | "dynamicsHttp" | "mssql";',
        "  requestQuerySchema?: JsonSchema;",
        "  requestBodySchema?: JsonSchema;",
        "  responseSchema?: JsonSchema;",
        "}",
        "",
        "function ep(",
        "  controller: string,",
        "  action: string,",
        "  method: ApiHttpMethod,",
        "  auth: ApiAuthMode,",
        "  summary: string,",
        "  extra: Partial<OnlineEndpointDef> & { path: string },",
        "): OnlineEndpointDef {",
        "  return {",
        "    controller,",
        "    action,",
        "    method,",
        "    auth,",
        "    summary,",
        '    impl: "proxy",',
        "    ...extra,",
        "  };",
        "}",
        "",
        "/** Full Sasfin.Wealth.Online.Api surface mirrored as BotFlow flows. */",
        "export const ONLINE_ENDPOINTS: OnlineEndpointDef[] = [",
    ]
    # special impls
    login_impls = {
        ("Account", "LoginAsync"): "login",
        ("Account", "RefreshAsync"): "refresh",
    }
    dynamics_controllers = {
        "Lookup",
        "SasSecLookup",
        "UserDetails",
        "IfaClients",
        "Proposal",
        "Case",
        "CommunicationCenter",
        "SasfinDocument",
        "AccountOnboarding",
        "ClientOnboarding",
        "AdvisorOnboarding",
    }
    mssql_controllers = {
        "PageContent",
        "Manage",
        "UserManagement",
        "FeeOverrideReport",
        "Support",
        "SasSecForm",
        "HubTicket",
        "Register",
        "Account",
        "IfaDashboard",
        "Market",
        "CaseTag",
    }

    for ep in endpoints:
        key = (ep["controller"], ep["action"])
        auth = "none" if key in ONLINE_PUBLIC else "jwt"
        extra: dict = {}
        if key in login_impls:
            extra["impl"] = login_impls[key]
        elif ep["controller"] in dynamics_controllers:
            extra["impl"] = "dynamicsHttp"
        elif ep["controller"] in mssql_controllers:
            extra["impl"] = "mssql"
        chunks.append(
            emit_endpoint(ep, auth=auth, extra=extra or None, extracted=extracted)
        )
    chunks.append("];")
    chunks.append("")
    chunks.append("export function onlinePath(ep: OnlineEndpointDef): string {")
    chunks.append("  return ep.path;")
    chunks.append("}")
    chunks.append("")
    return "\n".join(chunks)


def generate_dynamics(endpoints: list[dict], extracted: dict) -> str:
    chunks = [
        'import type { ApiAuthMode, ApiHttpMethod, JsonSchema } from "@botflow/shared";',
        'import { queryProps } from "../wealth/schemas.js";',
        "",
        "export interface DynamicsEndpointDef {",
        "  controller: string;",
        "  action: string;",
        "  method: ApiHttpMethod;",
        "  auth: ApiAuthMode;",
        "  summary: string;",
        "  path: string;",
        "  area?: string;",
        "  /** Dynamics CRM entity logical name used by the flow node. */",
        "  entity?: string;",
        '  impl?: "dynamics" | "proxy";',
        "  requestQuerySchema?: JsonSchema;",
        "  requestBodySchema?: JsonSchema;",
        "  responseSchema?: JsonSchema;",
        "}",
        "",
        "function ep(",
        "  controller: string,",
        "  action: string,",
        "  method: ApiHttpMethod,",
        "  auth: ApiAuthMode,",
        "  summary: string,",
        "  extra: Partial<DynamicsEndpointDef> & { path: string },",
        "): DynamicsEndpointDef {",
        "  return {",
        "    controller,",
        "    action,",
        "    method,",
        "    auth,",
        "    summary,",
        '    impl: "dynamics",',
        "    ...extra,",
        "  };",
        "}",
        "",
        "/** Full Sasfin.Dynamics.Api surface mirrored as BotFlow flows. */",
        "export const DYNAMICS_ENDPOINTS: DynamicsEndpointDef[] = [",
    ]
    for ep_item in endpoints:
        entity = DYNAMICS_ENTITY.get(ep_item["controller"], "contacts")
        chunks.append(
            emit_endpoint(
                ep_item,
                auth="apiKey",
                extra={"impl": "dynamics", "entity": entity},
                extracted=extracted,
            )
        )
    chunks.append("];")
    chunks.append("")
    chunks.append("export function dynamicsPath(ep: DynamicsEndpointDef): string {")
    chunks.append("  return ep.path;")
    chunks.append("}")
    chunks.append("")
    return "\n".join(chunks)


def main() -> None:
    online = json.loads(ONLINE_JSON.read_text(encoding="utf-8"))
    dyn = json.loads(DYN_JSON.read_text(encoding="utf-8"))
    schemas = (
        json.loads(SCHEMAS_JSON.read_text(encoding="utf-8"))
        if SCHEMAS_JSON.exists()
        else {}
    )
    OUT_ONLINE.parent.mkdir(parents=True, exist_ok=True)
    OUT_DYN.parent.mkdir(parents=True, exist_ok=True)
    OUT_ONLINE.write_text(
        generate_online(online, schemas.get("wealthOnline") or {}), encoding="utf-8"
    )
    OUT_DYN.write_text(
        generate_dynamics(dyn, schemas.get("dynamics") or {}), encoding="utf-8"
    )
    print(f"Wrote {OUT_ONLINE} ({len(online)} endpoints)")
    print(f"Wrote {OUT_DYN} ({len(dyn)} endpoints)")


if __name__ == "__main__":
    main()
