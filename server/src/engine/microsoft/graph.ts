import type { NodeContext } from "../types.js";

export type GraphTokenSource = {
  accessToken?: string;
  credentialId?: string;
  getCredential: NodeContext["getCredential"];
  getCredentialAsync: NodeContext["getCredentialAsync"];
  getCredentialData: NodeContext["getCredentialData"];
};

/** Resolve Graph token from param, env, or OAuth2-style stored credential. */
export async function getGraphToken(src: GraphTokenSource): Promise<string> {
  const inline = String(src.accessToken ?? "").trim();
  if (inline) return inline;

  const credentialId = String(src.credentialId ?? "").trim();
  if (credentialId) {
    const data = await src.getCredentialData(credentialId);
    const existing = String(
      data.accessToken ?? data.token ?? data.bearerToken ?? "",
    ).trim();
    if (existing) return existing;

    // Client credentials grant
    const tenant = String(data.tenantId ?? data.tenant ?? "common");
    const clientId = String(data.clientId ?? data.appId ?? "");
    const clientSecret = String(data.clientSecret ?? data.secret ?? "");
    const scope = String(
      data.scope ?? "https://graph.microsoft.com/.default",
    );
    if (clientId && clientSecret) {
      const body = new URLSearchParams({
        client_id: clientId,
        client_secret: clientSecret,
        scope,
        grant_type: "client_credentials",
      });
      const res = await fetch(
        `https://login.microsoftonline.com/${tenant}/oauth2/v2.0/token`,
        {
          method: "POST",
          headers: { "content-type": "application/x-www-form-urlencoded" },
          body,
        },
      );
      const json = (await res.json()) as {
        access_token?: string;
        error_description?: string;
      };
      if (!res.ok || !json.access_token) {
        throw new Error(
          json.error_description ||
            `Microsoft OAuth token request failed (${res.status})`,
        );
      }
      return json.access_token;
    }
  }

  return (
    (await src.getCredentialAsync("MS_GRAPH_TOKEN")) ||
    src.getCredential("MS_GRAPH_TOKEN") ||
    ""
  );
}

export async function graphFetch(
  path: string,
  token: string,
  method = "GET",
  body?: unknown,
): Promise<{ statusCode: number; body: unknown; headers: Record<string, string> }> {
  const url = path.startsWith("http")
    ? path
    : `https://graph.microsoft.com/v1.0${path.startsWith("/") ? path : `/${path}`}`;
  const res = await fetch(url, {
    method,
    headers: {
      authorization: `Bearer ${token}`,
      "content-type": "application/json",
      accept: "application/json",
    },
    body: body === undefined || method === "GET" || method === "DELETE"
      ? undefined
      : JSON.stringify(body),
  });
  const text = await res.text();
  let data: unknown = text;
  try {
    data = JSON.parse(text);
  } catch {
    /* keep */
  }
  return {
    statusCode: res.status,
    body: data,
    headers: Object.fromEntries(res.headers.entries()),
  };
}

/** Follow @odata.nextLink up to maxPages; flatten value arrays. */
export async function graphFetchAll(
  path: string,
  token: string,
  maxPages = 10,
): Promise<unknown[]> {
  const rows: unknown[] = [];
  let next: string | null = path;
  let page = 0;
  while (next && page < maxPages) {
    const result = await graphFetch(next, token, "GET");
    const body = result.body as { value?: unknown[]; "@odata.nextLink"?: string };
    if (Array.isArray(body?.value)) {
      rows.push(...body.value);
      next = body["@odata.nextLink"] ?? null;
    } else {
      rows.push(body);
      next = null;
    }
    page += 1;
  }
  return rows;
}

export function outlookPath(params: Record<string, unknown>): {
  path: string;
  method: string;
  body?: unknown;
} {
  const resource = String(params.resource ?? "message");
  const operation = String(
    params.operation ??
      (params.action === "sendMail" ? "send" : "getAll"),
  );

  if (resource === "message" || resource === "draft") {
    if (operation === "send" || params.action === "sendMail") {
      return {
        path: "/me/sendMail",
        method: "POST",
        body: params.message ?? {
          message: {
            subject: params.subject ?? "DataFlow",
            body: {
              contentType: "Text",
              content: String(params.body ?? ""),
            },
            toRecipients: [
              {
                emailAddress: { address: String(params.to ?? "") },
              },
            ],
          },
        },
      };
    }
    if (operation === "get") {
      const id = String(params.messageId ?? params.id ?? "");
      return { path: `/me/messages/${id}`, method: "GET" };
    }
    if (operation === "create") {
      return {
        path: resource === "draft" ? "/me/messages" : "/me/messages",
        method: "POST",
        body: params.message ?? {
          subject: params.subject ?? "DataFlow",
          body: {
            contentType: "Text",
            content: String(params.body ?? ""),
          },
        },
      };
    }
    return {
      path: String(params.path ?? "/me/messages?$top=25"),
      method: "GET",
    };
  }

  if (resource === "folder") {
    if (operation === "get") {
      return {
        path: `/me/mailFolders/${String(params.folderId ?? "")}`,
        method: "GET",
      };
    }
    return { path: "/me/mailFolders", method: "GET" };
  }

  return {
    path: String(params.path ?? "/me/messages?$top=10"),
    method: "GET",
  };
}

export function excelPath(params: Record<string, unknown>): {
  path: string;
  method: string;
} {
  const resource = String(params.resource ?? "workbook");
  const operation = String(params.operation ?? "list");
  const workbookId = String(params.workbookId ?? "");
  const worksheet = String(params.worksheetName ?? "Sheet1");
  const range = String(params.range ?? "A1:C10");

  if (params.path && String(params.path).trim()) {
    return { path: String(params.path), method: "GET" };
  }
  if (resource === "range" || operation === "getRange") {
    return {
      path: `/me/drive/items/${workbookId}/workbook/worksheets('${worksheet}')/range(address='${range}')`,
      method: "GET",
    };
  }
  if (resource === "worksheet") {
    return {
      path: `/me/drive/items/${workbookId}/workbook/worksheets`,
      method: "GET",
    };
  }
  if (operation === "get" && workbookId) {
    return { path: `/me/drive/items/${workbookId}`, method: "GET" };
  }
  return { path: "/me/drive/root/children", method: "GET" };
}

export function sharepointPath(params: Record<string, unknown>): {
  path: string;
  method: string;
} {
  const resource = String(params.resource ?? "site");
  const operation = String(params.operation ?? "getAll");
  const siteId = String(params.siteId ?? "");
  const listId = String(params.listId ?? "");

  if (resource === "list" && siteId) {
    if (operation === "get" && listId) {
      return { path: `/sites/${siteId}/lists/${listId}`, method: "GET" };
    }
    return { path: `/sites/${siteId}/lists`, method: "GET" };
  }
  if (resource === "item" && siteId && listId) {
    return {
      path: `/sites/${siteId}/lists/${listId}/items?$expand=fields`,
      method: "GET",
    };
  }
  if (resource === "file" && siteId) {
    return {
      path: `/sites/${siteId}/drive/root/children`,
      method: "GET",
    };
  }
  if (siteId && operation === "getChildren") {
    return { path: `/sites/${siteId}/drive/root/children`, method: "GET" };
  }
  if (siteId && operation === "get") {
    return { path: `/sites/${siteId}`, method: "GET" };
  }
  return {
    path: String(params.path ?? "/sites?search=*"),
    method: "GET",
  };
}
