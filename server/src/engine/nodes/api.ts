import type { NodeHandler } from "../types.js";
import { itemJson, makeItem, type WorkflowItem } from "../items.js";

async function buildAuthHeaders(
  auth: string,
  params: Record<string, unknown>,
  getCredential: (key: string) => string | undefined,
  getCredentialAsync: (key: string) => Promise<string | undefined>,
  getCredentialData: (id: string) => Promise<Record<string, unknown>>,
): Promise<Record<string, string>> {
  const headers: Record<string, string> = {};
  const credentialId = String(params.credentialId ?? "").trim();
  const credData = credentialId ? await getCredentialData(credentialId) : {};

  if (auth === "genericCredentialType" || auth === "headerAuth") {
    const name = String(
      credData.headerName ?? params.headerName ?? "Authorization",
    );
    const value =
      (await getCredentialAsync(
        String(params.credentialKey ?? (credentialId || "HTTP_AUTH_HEADER")),
      )) ??
      String(credData.value ?? credData.headerValue ?? params.headerValue ?? "");
    if (value) headers[name] = value;
  } else if (
    auth === "bearer" ||
    auth === "httpBearerAuth" ||
    auth === "predefinedCredentialType"
  ) {
    const token =
      String(credData.token ?? credData.accessToken ?? credData.apiKey ?? "") ||
      (await getCredentialAsync(
        String(params.credentialKey ?? (credentialId || "HTTP_BEARER_TOKEN")),
      )) ||
      String(params.bearerToken ?? "");
    if (token) headers.Authorization = `Bearer ${token}`;
  } else if (auth === "basicAuth" || auth === "basic") {
    const user =
      String(credData.user ?? credData.username ?? "") ||
      (await getCredentialAsync(
        String(params.credentialKeyUser ?? "HTTP_BASIC_USER"),
      )) ||
      String(params.user ?? "");
    const pass =
      String(credData.password ?? "") ||
      (await getCredentialAsync(
        String(params.credentialKeyPass ?? "HTTP_BASIC_PASS"),
      )) ||
      String(params.password ?? "");
    headers.Authorization = `Basic ${Buffer.from(`${user}:${pass}`).toString("base64")}`;
  } else if (auth === "oAuth2" || auth === "oauth2") {
    const token =
      String(credData.accessToken ?? credData.token ?? "") ||
      (credentialId
        ? await getCredentialAsync(`${credentialId}.accessToken`)
        : undefined);
    if (token) headers.Authorization = `Bearer ${token}`;
  } else {
    // none — still allow predefinedCredentialType-style id if set
    void getCredential;
  }
  return headers;
}

function buildUrl(
  base: string,
  query: Record<string, unknown> | undefined,
): string {
  if (!query || !Object.keys(query).length) return base;
  const u = new URL(base);
  for (const [k, v] of Object.entries(query)) {
    if (v === undefined || v === null) continue;
    u.searchParams.set(k, String(v));
  }
  return u.toString();
}

function nextPageUrl(
  mode: string,
  res: Response,
  data: unknown,
  currentUrl: string,
  page: number,
  params: Record<string, unknown>,
): string | null {
  if (mode === "off" || !mode) return null;
  if (mode === "nextUrl") {
    const path = String(params.nextUrlPath ?? "next");
    const body =
      data && typeof data === "object" && !Array.isArray(data)
        ? (data as Record<string, unknown>)
        : {};
    const linkHeader = res.headers.get("link") ?? "";
    const linkNext = linkHeader.match(/<([^>]+)>;\s*rel="?next"?/i)?.[1];
    const fromBody = body[path] ?? body.next ?? body.nextPage;
    const next = linkNext || (fromBody != null ? String(fromBody) : "");
    return next && next !== currentUrl ? next : null;
  }
  if (mode === "offset") {
    const limit = Number(params.paginationLimit ?? 100);
    const maxPages = Number(params.paginationMaxPages ?? 10);
    if (page + 1 >= maxPages) return null;
    const u = new URL(currentUrl);
    const offsetParam = String(params.offsetParam ?? "offset");
    const limitParam = String(params.limitParam ?? "limit");
    const prev = Number(u.searchParams.get(offsetParam) ?? 0);
    u.searchParams.set(offsetParam, String(prev + limit));
    u.searchParams.set(limitParam, String(limit));
    return u.toString();
  }
  return null;
}

export const httpRequestNode: NodeHandler = async ({
  node,
  items,
  resolve,
  getCredential,
  getCredentialAsync,
  getCredentialData,
}) => {
  const method = String(node.parameters.method ?? "GET").toUpperCase();
  const auth = String(node.parameters.authentication ?? "none");
  const bodyContentType = String(
    node.parameters.bodyContentType ?? node.parameters.contentType ?? "json",
  );
  const responseFormat = String(node.parameters.responseFormat ?? "json");
  const timeoutMs = Number(node.parameters.timeoutMs ?? 30_000);
  const paginationMode = String(node.parameters.paginationMode ?? "off");
  const maxPages = Math.max(1, Number(node.parameters.paginationMaxPages ?? 10));
  const results: WorkflowItem[] = [];

  const sources = items.length ? items : [makeItem({})];
  for (const item of sources) {
    const baseUrl = String(resolve(node.parameters.url ?? "", item));
    const query = resolve(
      (node.parameters.queryParameters as Record<string, unknown>) ??
        (node.parameters.qs as Record<string, unknown>) ??
        {},
      item,
    ) as Record<string, unknown>;
    let url = buildUrl(baseUrl, query);

    const headerParams = resolve(
      (node.parameters.headers as Record<string, string>) ?? {},
      item,
    ) as Record<string, string>;
    const authHeaders = await buildAuthHeaders(
      auth,
      node.parameters,
      getCredential,
      getCredentialAsync,
      getCredentialData,
    );

    let body: BodyInit | undefined;
    const headers: Record<string, string> = {
      ...authHeaders,
      ...headerParams,
    };
    const sendBody =
      node.parameters.sendBody !== false &&
      method !== "GET" &&
      method !== "HEAD";

    if (sendBody) {
      const bodyTemplate = node.parameters.body;
      if (
        bodyContentType === "form-data" ||
        bodyContentType === "multipart-form-data"
      ) {
        const form = new FormData();
        const params =
          (resolve(
            (node.parameters.bodyParameters as Record<string, unknown>) ??
              (typeof bodyTemplate === "object" && bodyTemplate
                ? bodyTemplate
                : {}),
            item,
          ) as Record<string, unknown>) ?? {};
        for (const [k, v] of Object.entries(params)) {
          form.append(k, String(v ?? ""));
        }
        body = form;
        delete headers["content-type"];
        delete headers["Content-Type"];
      } else if (bodyContentType === "form-urlencoded") {
        headers["content-type"] = "application/x-www-form-urlencoded";
        const form =
          (resolve(
            (node.parameters.bodyParameters as Record<string, unknown>) ??
              bodyTemplate ??
              {},
            item,
          ) as Record<string, unknown>) ?? {};
        body = new URLSearchParams(
          Object.entries(form).map(([k, v]) => [k, String(v ?? "")]),
        ).toString();
      } else if (bodyContentType === "raw" || bodyContentType === "text") {
        headers["content-type"] = String(
          node.parameters.rawContentType ?? "text/plain",
        );
        body = String(resolve(bodyTemplate ?? "", item));
      } else {
        headers["content-type"] = headers["content-type"] ?? "application/json";
        if (bodyTemplate === undefined || bodyTemplate === null) {
          body = undefined;
        } else if (bodyTemplate === "item") {
          body = JSON.stringify(itemJson(item));
        } else if (typeof bodyTemplate === "string") {
          const resolved = resolve(bodyTemplate, item);
          body =
            typeof resolved === "string"
              ? resolved
              : JSON.stringify(resolved);
        } else {
          body = JSON.stringify(resolve(bodyTemplate, item));
        }
      }
    }

    let page = 0;
    let currentUrl = url;
    while (page < maxPages) {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), timeoutMs);
      try {
        const res = await fetch(currentUrl, {
          method,
          headers,
          body: sendBody && page === 0 ? body : page === 0 ? body : undefined,
          signal: controller.signal,
          redirect:
            node.parameters.followRedirects === false ? "manual" : "follow",
        });

        const mime = res.headers.get("content-type") ?? "application/octet-stream";
        let data: unknown;
        let binary: Record<string, unknown> | undefined;

        if (responseFormat === "binary") {
          const buf = Buffer.from(await res.arrayBuffer());
          binary = {
            data: {
              data: buf.toString("base64"),
              mimeType: mime,
              fileSize: buf.byteLength,
              fileName: "response",
            },
          };
          data = {
            statusCode: res.status,
            mimeType: mime,
            fileSize: buf.byteLength,
          };
        } else {
          const text = await res.text();
          data = text;
          if (responseFormat === "json" || responseFormat === "autodetect") {
            try {
              data = JSON.parse(text);
            } catch {
              /* keep text */
            }
          }
        }

        const fullResponse = Boolean(node.parameters.fullResponse);
        const json = fullResponse
          ? {
              statusCode: res.status,
              headers: Object.fromEntries(res.headers.entries()),
              body: data,
            }
          : typeof data === "object" && data && !Array.isArray(data)
            ? (data as Record<string, unknown>)
            : { data };

        if (Array.isArray(data) && !fullResponse) {
          for (const row of data) {
            results.push(makeItem(row as Record<string, unknown>, item));
          }
        } else {
          const out = makeItem(json, item);
          if (binary) out.binary = binary;
          results.push(out);
        }

        const next = nextPageUrl(
          paginationMode,
          res,
          data,
          currentUrl,
          page,
          node.parameters,
        );
        if (!next) break;
        currentUrl = next;
        page += 1;
      } finally {
        clearTimeout(timer);
      }
    }
  }
  return { items: results };
};

function unwrapRespondBody(body: unknown): unknown {
  if (
    body &&
    typeof body === "object" &&
    !Array.isArray(body) &&
    "__raw" in (body as object)
  ) {
    return (body as { __raw: unknown }).__raw;
  }
  return body;
}

function isFullHttpResponse(
  value: unknown,
): value is { statusCode: number; body: unknown; headers?: Record<string, string> } {
  return (
    !!value &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    "statusCode" in value &&
    "body" in value
  );
}

export const webhookRespondNode: NodeHandler = async ({
  node,
  items,
}) => {
  const first = items[0] ? itemJson(items[0]) : undefined;
  if (isFullHttpResponse(first)) {
    return {
      items,
      respond: {
        statusCode: Number(first.statusCode),
        body: unwrapRespondBody(first.body),
      },
    };
  }
  const statusCode = Number(node.parameters.statusCode ?? 200);
  const responseData = String(node.parameters.responseData ?? "firstEntryJson");
  let body: unknown = node.parameters.body;
  if (responseData === "firstEntryJson") {
    body = body !== undefined ? body : first ?? {};
  } else if (responseData === "allEntries") {
    body = items.map((i) => itemJson(i));
  } else if (responseData === "noData") {
    body = undefined;
  }
  return {
    items,
    respond: { statusCode, body: unwrapRespondBody(body) },
  };
};

export const apiRespondNode: NodeHandler = webhookRespondNode;
