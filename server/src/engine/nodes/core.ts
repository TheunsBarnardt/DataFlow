import { createHmac, createHash, randomBytes, timingSafeEqual } from "node:crypto";
import { exec } from "node:child_process";
import { promisify } from "node:util";
import { writeFile, readFile } from "node:fs/promises";
import { gzipSync, gunzipSync, deflateSync, inflateSync } from "node:zlib";
import type { NodeHandler } from "../types.js";
import {
  ensureItems,
  getJsonPath,
  itemJson,
  makeItem,
  toItem,
  type WorkflowItem,
} from "../items.js";
import { getBinaryBuffer, setBinaryBuffer } from "../binaryUtil.js";

const execAsync = promisify(exec);

function requireCred(
  value: string | undefined,
  label: string,
): asserts value is string {
  if (!value) throw new Error(`${label} is required`);
}

export const noOpNode: NodeHandler = async ({ items }) => ({ items });

export const stickyNoteNode: NodeHandler = async ({ items }) => ({ items });

export const stopAndErrorNode: NodeHandler = async ({ node }) => {
  throw new Error(String(node.parameters.message ?? "Stopped by Stop and Error node"));
};

export const renameKeysNode: NodeHandler = async ({ node, items }) => {
  const mapping = (node.parameters.mapping as Record<string, string>) ?? {};
  return {
    items: items.map((item, idx) => {
      const next = { ...itemJson(item) };
      for (const [from, to] of Object.entries(mapping)) {
        if (from in next) {
          next[to] = next[from];
          if (from !== to) delete next[from];
        }
      }
      return makeItem(next, item, idx);
    }),
  };
};

/** Same-run dedupe; outputs kept + discarded. Supports allFieldsExcept. */
export const removeDuplicatesNode: NodeHandler = async ({ node, items }) => {
  const compare = String(node.parameters.compare ?? "selectedFields");
  const field = String(node.parameters.field ?? "");
  const fields =
    (node.parameters.fields as string[]) ??
    (field ? [field] : []);
  const exceptFields =
    (node.parameters.exceptFields as string[]) ??
    (node.parameters.fieldsToExclude as string[]) ??
    [];
  const removeOtherFields = Boolean(node.parameters.removeOtherFields);
  const seen = new Set<string>();
  const kept: WorkflowItem[] = [];
  const discarded: WorkflowItem[] = [];

  function keyOf(item: WorkflowItem): string {
    if (compare === "allFields" || (!fields.length && !field && compare !== "allFieldsExcept")) {
      return JSON.stringify(itemJson(item));
    }
    if (compare === "allFieldsExcept") {
      const src = { ...itemJson(item) };
      for (const f of exceptFields) delete src[f];
      return JSON.stringify(src);
    }
    const paths = fields.length ? fields : [field];
    return JSON.stringify(paths.map((p) => getJsonPath(item, p)));
  }

  for (const item of items) {
    const key = keyOf(item);
    if (seen.has(key)) {
      discarded.push(item);
      continue;
    }
    seen.add(key);
    if (removeOtherFields && fields.length) {
      const json: Record<string, unknown> = {};
      for (const f of fields) {
        const v = getJsonPath(item, f);
        if (v !== undefined) json[f] = v;
      }
      kept.push(makeItem(json, item));
    } else {
      kept.push(item);
    }
  }
  return {
    items: kept,
    outputs: { main: kept, kept, discarded },
  };
};

/** Split Out — explode array field(s) into items. */
export const splitOutNode: NodeHandler = async ({ node, items }) => {
  const fieldParam = String(node.parameters.field ?? node.parameters.fieldsToSplitOut ?? "data");
  const fieldsToSplit = fieldParam
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const include = String(node.parameters.include ?? "noOtherFields");
  const selectedOther =
    (node.parameters.selectedOtherFields as string[]) ??
    (node.parameters.includeFields as string[]) ??
    [];
  const destination = node.parameters.destinationFieldName
    ? String(node.parameters.destinationFieldName)
    : "";
  const destinations = destination
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const includeBinary = node.parameters.includeBinary !== false;
  const out: WorkflowItem[] = [];

  for (const item of items) {
    // Binary split mode
    if (fieldsToSplit.includes("$binary") && item.binary) {
      for (const [binKey, binVal] of Object.entries(item.binary)) {
        const json =
          include === "allOtherFields"
            ? { ...itemJson(item), binaryKey: binKey, binary: binVal }
            : { binaryKey: binKey, binary: binVal };
        const made = makeItem(json, includeBinary ? item : undefined);
        if (!includeBinary) delete made.binary;
        else made.binary = { [binKey]: binVal };
        out.push(made);
      }
      continue;
    }

    // Multi-field: zip by index
    const arrays = fieldsToSplit.map((f) => {
      let arr = getJsonPath(item, f);
      if (arr == null) return [];
      if (!Array.isArray(arr)) {
        if (arr && typeof arr === "object") arr = Object.values(arr);
        else arr = [arr];
      }
      return arr as unknown[];
    });
    const maxLen = Math.max(0, ...arrays.map((a) => a.length));
    const parent = itemJson(item);

    for (let i = 0; i < maxLen; i++) {
      let json: Record<string, unknown> = {};
      if (include === "allOtherFields") {
        json = { ...parent };
        for (const f of fieldsToSplit) delete json[f.split(".")[0]!];
      } else if (include === "selectedOtherFields") {
        for (const f of selectedOther) {
          const v = getJsonPath(item, f);
          if (v !== undefined) json[f] = v;
        }
      }

      fieldsToSplit.forEach((f, fi) => {
        const el = arrays[fi]?.[i];
        const dest = destinations[fi] || (fieldsToSplit.length === 1 ? destination : "");
        if (dest) json[dest] = el;
        else if (el && typeof el === "object" && !Array.isArray(el) && fieldsToSplit.length === 1) {
          Object.assign(json, el as Record<string, unknown>);
        } else {
          json[f.includes(".") ? f.split(".").pop()! : f] = el;
        }
      });

      const made = makeItem(json, includeBinary ? item : undefined);
      if (!includeBinary) delete made.binary;
      out.push(made);
    }
  }
  return { items: out };
};

/** Summarize — aggregations + optional groupBy. */
export const summarizeNode: NodeHandler = async ({ node, items }) => {
  const field = String(node.parameters.field ?? "value");
  const fields = (node.parameters.fields as string[]) ?? [field];
  const aggregations =
    (node.parameters.aggregations as Array<{
      field?: string;
      aggregation?: string;
      outputFieldName?: string;
    }>) ?? [];
  const groupBy = String(node.parameters.groupBy ?? "").trim();

  const aggList =
    aggregations.length > 0
      ? aggregations
      : fields.flatMap((f) => [
          { field: f, aggregation: "sum", outputFieldName: `${f}_sum` },
          { field: f, aggregation: "average", outputFieldName: `${f}_avg` },
        ]);

  function aggregate(groupItems: WorkflowItem[]): Record<string, unknown> {
    const row: Record<string, unknown> = { count: groupItems.length };
    for (const agg of aggList) {
      const f = String(agg.field ?? field);
      const op = String(agg.aggregation ?? "sum");
      const name = String(agg.outputFieldName ?? `${f}_${op}`);
      const nums = groupItems
        .map((i) => Number(getJsonPath(i, f)))
        .filter((n) => !Number.isNaN(n));
      switch (op) {
        case "count":
          row[name] = groupItems.length;
          break;
        case "countUnique":
          row[name] = new Set(groupItems.map((i) => getJsonPath(i, f))).size;
          break;
        case "sum":
          row[name] = nums.reduce((a, b) => a + b, 0);
          break;
        case "average":
        case "avg":
          row[name] = nums.length
            ? nums.reduce((a, b) => a + b, 0) / nums.length
            : 0;
          break;
        case "min":
          row[name] = nums.length ? Math.min(...nums) : 0;
          break;
        case "max":
          row[name] = nums.length ? Math.max(...nums) : 0;
          break;
        default:
          row[name] = nums.reduce((a, b) => a + b, 0);
      }
    }
    return row;
  }

  if (!groupBy) {
    return { items: [makeItem(aggregate(items))] };
  }

  const groups = new Map<string, WorkflowItem[]>();
  for (const item of items) {
    const key = String(getJsonPath(item, groupBy) ?? "");
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(item);
  }
  return {
    items: [...groups.entries()].map(([key, groupItems]) =>
      makeItem({ [groupBy]: key, ...aggregate(groupItems) }),
    ),
  };
};

/** Compare Datasets — dual-input (handles 0/1) preferred; falls back to split. */
export const compareDatasetsNode: NodeHandler = async ({
  node,
  items,
  inputsByHandle,
}) => {
  const field = String(node.parameters.field ?? "id");
  let a =
    inputsByHandle["0"] ??
    inputsByHandle.input1 ??
    inputsByHandle.main ??
    [];
  let b = inputsByHandle["1"] ?? inputsByHandle.input2 ?? [];
  if (!b.length && a.length === items.length && items.length > 1) {
    const mid = Math.ceil(items.length / 2);
    a = items.slice(0, mid);
    b = items.slice(mid);
  } else if (!a.length) {
    a = items;
  }
  const aKeys = new Set(a.map((i) => String(getJsonPath(i, field) ?? "")));
  const bKeys = new Set(b.map((i) => String(getJsonPath(i, field) ?? "")));
  const onlyA = a.filter(
    (i) => !bKeys.has(String(getJsonPath(i, field) ?? "")),
  );
  const onlyB = b.filter(
    (i) => !aKeys.has(String(getJsonPath(i, field) ?? "")),
  );
  const both = a.filter((i) =>
    bKeys.has(String(getJsonPath(i, field) ?? "")),
  );
  return {
    items: both,
    outputs: {
      main: both,
      both,
      onlyA,
      onlyB,
    },
  };
};

export const compressionNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "compress");
  const format = String(node.parameters.format ?? "gzip"); // gzip | deflate
  const property = String(node.parameters.binaryPropertyName ?? "data");
  const out: WorkflowItem[] = [];

  for (const item of items) {
    let buf = getBinaryBuffer(item, property);
    if (!buf) {
      const field = String(node.parameters.field ?? "content");
      const raw = getJsonPath(item, field);
      if (raw == null) {
        throw new Error(
          "Compression requires binary data or a text field (parameters.field)",
        );
      }
      buf = Buffer.from(typeof raw === "string" ? raw : JSON.stringify(raw), "utf8");
    }

    let result: Buffer;
    if (op === "decompress" || op === "gunzip") {
      result =
        format === "deflate" ? inflateSync(buf) : gunzipSync(buf);
      try {
        const text = result.toString("utf8");
        const parsed = JSON.parse(text) as unknown;
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
          out.push(makeItem(parsed as Record<string, unknown>, item));
          continue;
        }
        out.push(
          makeItem({ ...itemJson(item), content: text, compression: op }, item),
        );
        continue;
      } catch {
        out.push(
          setBinaryBuffer(
            makeItem({ ...itemJson(item), compression: op }, item),
            result,
            { property, mimeType: "application/octet-stream" },
          ),
        );
        continue;
      }
    }

    result = format === "deflate" ? deflateSync(buf) : gzipSync(buf);
    out.push(
      setBinaryBuffer(
        makeItem(
          {
            ...itemJson(item),
            compression: op,
            format,
            bytes: result.byteLength,
          },
          item,
        ),
        result,
        {
          property,
          mimeType:
            format === "deflate" ? "application/zlib" : "application/gzip",
          fileName: `${property}.${format === "deflate" ? "zz" : "gz"}`,
        },
      ),
    );
  }
  return { items: out };
};

export const convertToFileNode: NodeHandler = async ({ node, items }) => {
  const fileName = String(node.parameters.fileName ?? "output.json");
  const encoding = String(node.parameters.encoding ?? "utf8") as BufferEncoding;
  const content =
    typeof node.parameters.content === "string"
      ? node.parameters.content
      : JSON.stringify(items, null, 2);
  const filePath = String(node.parameters.path ?? fileName);
  await writeFile(filePath, content, encoding);
  return { items: [{ filePath, bytes: Buffer.byteLength(content, encoding) }] };
};

export const extractFromFileNode: NodeHandler = async ({ node }) => {
  const filePath = String(node.parameters.path ?? "");
  const encoding = String(node.parameters.encoding ?? "utf8") as BufferEncoding;
  const content = await readFile(filePath, encoding);
  try {
    return { items: [toItem(JSON.parse(content))] };
  } catch {
    return { items: [makeItem({ content })] };
  }
};

export const markdownNode: NodeHandler = async ({ node, items }) => {
  const field = String(node.parameters.field ?? "text");
  const mode = String(node.parameters.mode ?? "toHtml");
  return {
    items: items.map((item, idx) => {
      const text = String(getJsonPath(item, field) ?? "");
      if (mode === "toHtml") {
        const html = text
          .replace(/^### (.*$)/gim, "<h3>$1</h3>")
          .replace(/^## (.*$)/gim, "<h2>$1</h2>")
          .replace(/^# (.*$)/gim, "<h1>$1</h1>")
          .replace(/\*\*(.*?)\*\*/gim, "<strong>$1</strong>")
          .replace(/\*(.*?)\*/gim, "<em>$1</em>")
          .replace(/\n/g, "<br/>");
        return makeItem({ ...itemJson(item), html }, item, idx);
      }
      return makeItem({ ...itemJson(item), markdown: text }, item, idx);
    }),
  };
};

export const htmlExtractNode: NodeHandler = async ({ node, items }) => {
  const selector = String(node.parameters.selector ?? "title");
  return {
    items: items.map((item, idx) => {
      const json = itemJson(item);
      const html = String(json.html ?? json.body ?? "");
      const re =
        selector === "title"
          ? /<title[^>]*>([\s\S]*?)<\/title>/i
          : new RegExp(`<${selector}[^>]*>([\\s\\S]*?)<\\/${selector}>`, "i");
      const match = html.match(re);
      return makeItem(
        { ...json, extracted: match?.[1]?.trim() ?? null },
        item,
        idx,
      );
    }),
  };
};

export const executeCommandNode: NodeHandler = async ({ node }) => {
  const command = String(node.parameters.command ?? "echo dataflow");
  const { stdout, stderr } = await execAsync(command, {
    timeout: Number(node.parameters.timeoutMs ?? 30_000),
    windowsHide: true,
  });
  return { items: [{ stdout, stderr, command }] };
};

export const executeWorkflowNode: NodeHandler = async ({ node, items }) => {
  const workflowId = String(node.parameters.workflowId ?? "");
  if (!workflowId) {
    return { items: [{ warning: "workflowId required", input: items }] };
  }
  // Dynamic import avoids circular dependency with executor → registry → core
  const { executeWorkflow } = await import("../executor.js");
  const run = await executeWorkflow({
    workflowId,
    trigger: "executeWorkflow",
    input: node.parameters.input ?? items,
  });
  return { items: [{ runId: run._id, status: run.status, output: run.output }] };
};

export const graphqlNode: NodeHandler = async ({ node, getCredential }) => {
  const url = String(node.parameters.url ?? "");
  const query = String(node.parameters.query ?? "{ __typename }");
  const variables = (node.parameters.variables as Record<string, unknown>) ?? {};
  const token =
    String(node.parameters.accessToken ?? "") ||
    getCredential("GRAPHQL_TOKEN") ||
    "";
  if (!url) return { items: [{ warning: "GraphQL url required" }] };
  const res = await fetch(url, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify({ query, variables }),
  });
  const body = await res.json();
  return { items: [{ statusCode: res.status, body }] };
};

export const jwtNode: NodeHandler = async ({ node, items, getCredential }) => {
  const op = String(node.parameters.operation ?? "sign");
  const secret =
    String(node.parameters.secret ?? "") ||
    getCredential("JWT_SECRET") ||
    "dataflow-dev-secret";
  if (op === "verify") {
    const token = String(
      node.parameters.token ?? (items[0] ? itemJson(items[0]).token : "") ?? "",
    );
    const [h, p, s] = token.split(".");
    if (!h || !p || !s) throw new Error("Invalid JWT");
    const expected = createHmac("sha256", secret)
      .update(`${h}.${p}`)
      .digest("base64url");
    const a = Buffer.from(s);
    const b = Buffer.from(expected);
    const ok = a.length === b.length && timingSafeEqual(a, b);
    if (!ok) throw new Error("JWT signature invalid");
    return {
      items: [
        makeItem({
          payload: JSON.parse(Buffer.from(p, "base64url").toString("utf8")),
        }),
      ],
    };
  }
  const payload =
    (node.parameters.payload as Record<string, unknown>) ??
    (items[0] ? itemJson(items[0]) : {});
  const header = Buffer.from(
    JSON.stringify({ alg: "HS256", typ: "JWT" }),
  ).toString("base64url");
  const body = Buffer.from(JSON.stringify(payload)).toString("base64url");
  const sig = createHmac("sha256", secret)
    .update(`${header}.${body}`)
    .digest("base64url");
  return {
    items: [makeItem({ token: `${header}.${body}.${sig}`, payload })],
  };
};

export const formNode: NodeHandler = async ({ node, items }) => ({
  items: items.length
    ? items
    : [{ formId: String(node.parameters.formId ?? "form"), fields: node.parameters.fields ?? {} }],
});

export const rssReadNode: NodeHandler = async ({ node }) => {
  const url = String(node.parameters.url ?? "");
  if (!url) return { items: [{ warning: "RSS url required" }] };
  const res = await fetch(url);
  const xml = await res.text();
  const titles = [...xml.matchAll(/<title>([\s\S]*?)<\/title>/gi)].map((m) =>
    m[1].replace(/<!\[CDATA\[|\]\]>/g, "").trim(),
  );
  return {
    items: titles.slice(1, 21).map((title, i) => ({ title, index: i, source: url })),
  };
};

export const sendEmailNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  getCredentialData,
  resolve,
}) => {
  const first = items[0] ?? makeItem({});
  const cred = await getCredentialData(
    String(node.parameters.credentialId ?? ""),
  );
  const to = String(resolve(node.parameters.to ?? "", first));
  const subject = String(
    resolve(node.parameters.subject ?? "DataFlow", first),
  );
  const text = String(
    resolve(
      node.parameters.text ??
        node.parameters.message ??
        JSON.stringify(items.map((i) => itemJson(i))),
      first,
    ),
  );
  const html = node.parameters.html
    ? String(resolve(node.parameters.html, first))
    : undefined;
  const cc = node.parameters.cc
    ? String(resolve(node.parameters.cc, first))
    : undefined;
  const bcc = node.parameters.bcc
    ? String(resolve(node.parameters.bcc, first))
    : undefined;
  const from =
    String(resolve(node.parameters.from ?? "", first)) ||
    String(cred.from ?? "") ||
    getCredential("SMTP_FROM") ||
    "dataflow@localhost";

  const smtpUrl =
    String(node.parameters.smtpUrl ?? "") ||
    String(cred.smtpUrl ?? cred.url ?? "") ||
    getCredential("SMTP_URL") ||
    "";
  const host =
    String(node.parameters.host ?? "") ||
    String(cred.host ?? "") ||
    getCredential("SMTP_HOST") ||
    "";
  const user =
    String(node.parameters.user ?? "") ||
    String(cred.user ?? cred.username ?? "") ||
    getCredential("SMTP_USER") ||
    "";
  const pass =
    String(node.parameters.password ?? "") ||
    String(cred.password ?? "") ||
    getCredential("SMTP_PASSWORD") ||
    "";

  if (!to) throw new Error("Send Email: 'to' is required");

  const nodemailer = (await import("nodemailer")).default;
  let transport;
  if (smtpUrl) {
    transport = nodemailer.createTransport(smtpUrl);
  } else if (host) {
    transport = nodemailer.createTransport({
      host,
      port: Number(node.parameters.port ?? getCredential("SMTP_PORT") ?? 587),
      secure: Boolean(node.parameters.secure),
      auth: user ? { user, pass } : undefined,
    });
  } else {
    throw new Error(
      "Send Email: set SMTP_URL or host/user credentials (SMTP_HOST / SMTP_USER / SMTP_PASSWORD)",
    );
  }

  const info = await transport.sendMail({
    from,
    to,
    cc,
    bcc,
    subject,
    text,
    html,
  });
  return {
    items: [
      makeItem({
        accepted: info.accepted,
        rejected: info.rejected,
        messageId: info.messageId,
        to,
        subject,
      }),
    ],
  };
};

export const ftpNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  getCredentialData,
  resolve,
}) => {
  const first = items[0] ?? makeItem({});
  const cred = await getCredentialData(
    String(node.parameters.credentialId ?? ""),
  );
  const host =
    String(resolve(node.parameters.host ?? "", first)) ||
    String(cred.host ?? "") ||
    getCredential("FTP_HOST") ||
    "";
  requireCred(host, "FTP host");
  const user =
    String(resolve(node.parameters.user ?? "", first)) ||
    String(cred.user ?? cred.username ?? "") ||
    getCredential("FTP_USER") ||
    "";
  const password =
    String(resolve(node.parameters.password ?? "", first)) ||
    String(cred.password ?? "") ||
    getCredential("FTP_PASSWORD") ||
    "";
  const remotePath = String(
    resolve(node.parameters.path ?? node.parameters.remotePath ?? "/", first),
  );
  const op = String(node.parameters.operation ?? "list");
  const secure = Boolean(
    node.parameters.secure ?? node.parameters.ftps ?? cred.secure,
  );
  const port = Number(
    node.parameters.port ?? cred.port ?? (secure ? 990 : 21),
  );

  const { Client: FtpClient } = await import("basic-ftp");
  const client = new FtpClient();
  try {
    await client.access({
      host,
      port,
      user: user || undefined,
      password: password || undefined,
      secure,
    });

    if (op === "list") {
      const list = await client.list(remotePath);
      return {
        items: list.map((e) =>
          makeItem({
            name: e.name,
            type: e.type,
            size: e.size,
            modifiedAt: e.modifiedAt,
            path: remotePath,
          }),
        ),
      };
    }

    if (op === "download") {
      const { Writable } = await import("node:stream");
      const chunks: Buffer[] = [];
      const writable = new Writable({
        write(chunk, _enc, cb) {
          chunks.push(Buffer.from(chunk));
          cb();
        },
      });
      await client.downloadTo(writable, remotePath);
      const buf = Buffer.concat(chunks);
      return {
        items: [
          setBinaryBuffer(
            makeItem({ path: remotePath, bytes: buf.byteLength }),
            buf,
            {
              fileName: remotePath.split("/").pop() ?? "download",
              mimeType: "application/octet-stream",
            },
          ),
        ],
      };
    }

    if (op === "upload") {
      const localPath = String(
        resolve(node.parameters.localPath ?? "", first) || "",
      );
      const { Readable } = await import("node:stream");
      if (localPath) {
        await client.uploadFrom(localPath, remotePath);
      } else {
        const bin = getBinaryBuffer(first);
        const content =
          bin ??
          Buffer.from(
            String(
              resolve(
                node.parameters.content ??
                  JSON.stringify(itemJson(first), null, 2),
                first,
              ),
            ),
            "utf8",
          );
        await client.uploadFrom(Readable.from(content), remotePath);
      }
      return {
        items: [makeItem({ uploaded: true, path: remotePath, host })],
      };
    }

    throw new Error(`FTP unsupported operation: ${op}`);
  } finally {
    client.close();
  }
};

export const sshNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  getCredentialData,
  resolve,
}) => {
  const first = items[0] ?? makeItem({});
  const cred = await getCredentialData(
    String(node.parameters.credentialId ?? ""),
  );
  const host =
    String(resolve(node.parameters.host ?? "", first)) ||
    String(cred.host ?? "") ||
    getCredential("SSH_HOST") ||
    "";
  requireCred(host, "SSH host");
  const username =
    String(resolve(node.parameters.username ?? node.parameters.user ?? "", first)) ||
    String(cred.username ?? cred.user ?? "") ||
    getCredential("SSH_USER") ||
    "";
  requireCred(username, "SSH username");
  const password =
    String(resolve(node.parameters.password ?? "", first)) ||
    String(cred.password ?? "") ||
    getCredential("SSH_PASSWORD") ||
    "";
  const privateKey =
    String(resolve(node.parameters.privateKey ?? "", first)) ||
    String(cred.privateKey ?? "") ||
    getCredential("SSH_PRIVATE_KEY") ||
    "";
  const command = String(
    resolve(node.parameters.command ?? "uname -a", first),
  );
  const port = Number(node.parameters.port ?? cred.port ?? 22);

  if (!password && !privateKey) {
    throw new Error("SSH: password or privateKey credential required");
  }

  const { Client: SshClient } = await import("ssh2");
  const result = await new Promise<{
    stdout: string;
    stderr: string;
    code: number | null;
  }>((resolvePromise, reject) => {
    const client = new SshClient();
    client
      .on("ready", () => {
        client.exec(command, (err, stream) => {
          if (err) {
            client.end();
            reject(err);
            return;
          }
          let stdout = "";
          let stderr = "";
          stream
            .on("close", (code: number | null) => {
              client.end();
              resolvePromise({ stdout, stderr, code });
            })
            .on("data", (d: Buffer) => {
              stdout += d.toString("utf8");
            });
          stream.stderr.on("data", (d: Buffer) => {
            stderr += d.toString("utf8");
          });
        });
      })
      .on("error", reject)
      .connect({
        host,
        port,
        username,
        password: password || undefined,
        privateKey: privateKey || undefined,
      });
  });

  return {
    items: [
      makeItem({
        stdout: result.stdout,
        stderr: result.stderr,
        code: result.code,
        command,
        host,
      }),
    ],
  };
};

export const gitNode: NodeHandler = async ({ node }) => {
  const command = String(node.parameters.command ?? "git status --short");
  const cwd = node.parameters.cwd ? String(node.parameters.cwd) : undefined;
  const { stdout, stderr } = await execAsync(command, {
    cwd,
    timeout: 30_000,
    windowsHide: true,
  });
  return { items: [{ stdout, stderr, command }] };
};

export const totpNode: NodeHandler = async ({ node, getCredential }) => {
  const secret =
    String(node.parameters.secret ?? "") ||
    getCredential("TOTP_SECRET") ||
    "";
  requireCred(secret, "TOTP secret");
  const { Secret, TOTP } = await import("otpauth");
  let secretObj: InstanceType<typeof Secret>;
  try {
    secretObj = Secret.fromBase32(secret.replace(/\s+/g, ""));
  } catch {
    secretObj = Secret.fromUTF8(secret);
  }
  const totp = new TOTP({
    secret: secretObj,
    digits: Number(node.parameters.digits ?? 6),
    period: Number(node.parameters.period ?? 30),
    algorithm: String(node.parameters.algorithm ?? "SHA1"),
    label: String(node.parameters.label ?? "DataFlow"),
  });
  const token = totp.generate();
  return {
    items: [
      makeItem({
        token,
        period: totp.period,
        digits: totp.digits,
        remaining: totp.period - (Math.floor(Date.now() / 1000) % totp.period),
      }),
    ],
  };
};

export const timeSavedNode: NodeHandler = async ({ node, items }) => {
  const minutes = Number(node.parameters.minutesSaved ?? 5);
  const mode = String(node.parameters.mode ?? "once");
  const minutesSaved =
    mode === "perItem" ? minutes * Math.max(items.length, 1) : minutes;
  return {
    items: [{ minutesSaved, itemsProcessed: items.length, mode }],
  };
};

export const dataTableNode: NodeHandler = async ({ node, items }) => {
  const first = items[0] ? itemJson(items[0]) : {};
  const columns = (node.parameters.columns as string[]) ?? Object.keys(first);
  return {
    items: [
      makeItem({
        columns,
        rows: items.map((item) => {
          const json = itemJson(item);
          return Object.fromEntries(columns.map((c) => [c, json[c]]));
        }),
      }),
    ],
  };
};

export const executionDataNode: NodeHandler = async ({ items, runInput }) => ({
  items: [{ runInput, itemCount: items.length, at: new Date().toISOString() }],
});

export const simulateNode: NodeHandler = async ({ node, items }) => {
  const output = node.parameters.output;
  if (Array.isArray(output) && output.length) return { items: ensureItems(output) };
  if (items.length) return { items };
  return { items: [makeItem({ simulated: true })] };
};

export const aiTransformNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  resolve,
}) => {
  const prompt = String(node.parameters.prompt ?? "transform");
  const apiKey =
    String(node.parameters.apiKey ?? "") ||
    getCredential("OPENAI_API_KEY") ||
    "";
  requireCred(apiKey, "OPENAI_API_KEY");
  const baseUrl = (
    String(node.parameters.baseUrl ?? "") ||
    getCredential("OPENAI_BASE_URL") ||
    "https://api.openai.com/v1"
  ).replace(/\/$/, "");
  const model =
    String(node.parameters.model ?? "") ||
    getCredential("OPENAI_MODEL") ||
    "gpt-4o-mini";

  const out: WorkflowItem[] = [];
  for (const item of items) {
    const itemPrompt = String(resolve(prompt, item));
    const userContent = `${itemPrompt}\n\nInput JSON:\n${JSON.stringify(itemJson(item), null, 2)}\n\nRespond with a single JSON object only.`;
    const res = await fetch(`${baseUrl}/chat/completions`, {
      method: "POST",
      headers: {
        authorization: `Bearer ${apiKey}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({
        model,
        temperature: Number(node.parameters.temperature ?? 0),
        response_format: { type: "json_object" },
        messages: [
          {
            role: "system",
            content:
              "You transform workflow items. Return only valid JSON matching the requested transform.",
          },
          { role: "user", content: userContent },
        ],
      }),
    });
    const body = (await res.json()) as {
      error?: { message?: string };
      choices?: Array<{ message?: { content?: string } }>;
    };
    if (!res.ok) {
      throw new Error(
        body.error?.message ?? `AI Transform HTTP ${res.status}`,
      );
    }
    const content = body.choices?.[0]?.message?.content ?? "{}";
    let parsed: Record<string, unknown>;
    try {
      parsed = JSON.parse(content) as Record<string, unknown>;
    } catch {
      parsed = { ...itemJson(item), aiText: content };
    }
    out.push(makeItem(parsed, item));
  }
  return { items: out };
};

export const n8nApiNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  resolve,
}) => {
  const first = items[0] ?? makeItem({});
  const baseUrl = (
    String(resolve(node.parameters.baseUrl ?? "", first)) ||
    getCredential("N8N_BASE_URL") ||
    ""
  ).replace(/\/$/, "");
  requireCred(baseUrl, "n8n baseUrl (N8N_BASE_URL)");
  const apiKey =
    String(resolve(node.parameters.apiKey ?? "", first)) ||
    getCredential("N8N_API_KEY") ||
    "";
  requireCred(apiKey, "n8n API key (N8N_API_KEY)");
  const operation = String(node.parameters.operation ?? "getWorkflows");
  const workflowId = String(
    resolve(node.parameters.workflowId ?? "", first) || "",
  );

  let path = "/api/v1/workflows";
  if (operation === "getExecutions") path = "/api/v1/executions";
  else if (operation === "getWorkflow") {
    if (!workflowId) throw new Error("n8n getWorkflow requires workflowId");
    path = `/api/v1/workflows/${encodeURIComponent(workflowId)}`;
  } else if (operation === "getExecution") {
    const executionId = String(
      resolve(node.parameters.executionId ?? "", first) || "",
    );
    if (!executionId) throw new Error("n8n getExecution requires executionId");
    path = `/api/v1/executions/${encodeURIComponent(executionId)}`;
  }

  const res = await fetch(`${baseUrl}${path}`, {
    headers: {
      "X-N8N-API-KEY": apiKey,
      accept: "application/json",
    },
  });
  const text = await res.text();
  let data: unknown = text;
  try {
    data = JSON.parse(text);
  } catch {
    /* keep */
  }
  if (!res.ok) {
    throw new Error(
      typeof data === "object" && data && "message" in data
        ? String((data as { message: unknown }).message)
        : `n8n API HTTP ${res.status}`,
    );
  }

  if (
    data &&
    typeof data === "object" &&
    Array.isArray((data as { data?: unknown }).data)
  ) {
    return {
      items: ensureItems((data as { data: unknown[] }).data),
    };
  }
  return {
    items: [
      makeItem(
        typeof data === "object" && data
          ? (data as Record<string, unknown>)
          : { data },
      ),
    ],
  };
};

export const readPdfNode: NodeHandler = async ({ node, items }) => {
  const filePath = String(node.parameters.path ?? "");
  let buf: Buffer;
  if (filePath) {
    buf = await readFile(filePath);
  } else {
    const fromItem = items[0] ? getBinaryBuffer(items[0]) : null;
    if (!fromItem) {
      throw new Error("Read PDF: path or binary input required");
    }
    buf = fromItem;
  }
  const { PDFParse } = await import("pdf-parse");
  const parser = new PDFParse({ data: buf });
  try {
    const textResult = await parser.getText();
    const infoResult = await parser.getInfo().catch(() => null);
    return {
      items: [
        makeItem({
          path: filePath || undefined,
          text: textResult.text,
          pages: textResult.total,
          info: infoResult ?? undefined,
          bytes: buf.byteLength,
        }),
      ],
    };
  } finally {
    await parser.destroy().catch(() => undefined);
  }
};

export const moveBinaryDataNode: NodeHandler = async ({ node, items }) => {
  const mode = String(node.parameters.mode ?? "jsonToBinary");
  return {
    items: items.map((item, idx) => {
      if (mode === "jsonToBinary") {
        const data = Buffer.from(JSON.stringify(itemJson(item))).toString(
          "base64",
        );
        return {
          ...makeItem(itemJson(item), item, idx),
          binary: { data, mimeType: "application/json" },
        };
      }
      const b64 = String(item.binary?.data ?? "");
      try {
        return toItem(
          JSON.parse(Buffer.from(b64, "base64").toString("utf8")),
          idx,
        );
      } catch {
        return makeItem(
          {
            ...itemJson(item),
            text: Buffer.from(b64, "base64").toString("utf8"),
          },
          item,
          idx,
        );
      }
    }),
  };
};

export const editImageNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "noop");
  const property = String(node.parameters.binaryPropertyName ?? "data");
  const out: WorkflowItem[] = [];

  for (const item of items) {
    if (op === "noop") {
      out.push(item);
      continue;
    }
    const buf = getBinaryBuffer(item, property);
    if (!buf) {
      throw new Error("Edit Image requires binary input");
    }
    const sharp = (await import("sharp")).default;
    let img = sharp(buf);
    if (op === "resize") {
      img = img.resize({
        width: node.parameters.width
          ? Number(node.parameters.width)
          : undefined,
        height: node.parameters.height
          ? Number(node.parameters.height)
          : undefined,
        fit: (String(node.parameters.fit ?? "inside") as
          | "cover"
          | "contain"
          | "fill"
          | "inside"
          | "outside"),
      });
    } else if (op === "crop") {
      img = img.extract({
        left: Number(node.parameters.left ?? 0),
        top: Number(node.parameters.top ?? 0),
        width: Number(node.parameters.width ?? 100),
        height: Number(node.parameters.height ?? 100),
      });
    } else if (op === "format") {
      const fmt = String(node.parameters.format ?? "png") as
        | "png"
        | "jpeg"
        | "webp"
        | "avif";
      img = img.toFormat(fmt);
    } else {
      throw new Error(`Edit Image unsupported operation: ${op}`);
    }
    const result = await img.toBuffer({ resolveWithObject: true });
    out.push(
      setBinaryBuffer(
        makeItem(
          {
            ...itemJson(item),
            editImage: {
              operation: op,
              width: result.info.width,
              height: result.info.height,
              format: result.info.format,
            },
          },
          item,
        ),
        result.data,
        {
          property,
          mimeType: `image/${result.info.format}`,
          fileName: `image.${result.info.format}`,
        },
      ),
    );
  }
  return { items: out };
};

export const icalendarNode: NodeHandler = async ({ node, items }) => {
  const title = String(node.parameters.title ?? "DataFlow Event");
  const start = String(node.parameters.start ?? new Date().toISOString());
  const ics = [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "BEGIN:VEVENT",
    `SUMMARY:${title}`,
    `DTSTART:${start.replace(/[-:]/g, "").replace(/\.\d{3}/, "")}`,
    "END:VEVENT",
    "END:VCALENDAR",
  ].join("\r\n");
  const base = items[0] ? itemJson(items[0]) : {};
  return { items: [makeItem({ ...base, ics, title, start }, items[0])] };
};

export const ldapNode: NodeHandler = async ({
  node,
  items,
  getCredential,
  resolve,
}) => {
  const first = items[0] ?? makeItem({});
  const host =
    String(resolve(node.parameters.host ?? "", first)) ||
    getCredential("LDAP_URL") ||
    getCredential("LDAP_HOST") ||
    "";
  requireCred(host, "LDAP host/url");
  const url = host.includes("://") ? host : `ldap://${host}`;
  const bindDN =
    String(resolve(node.parameters.bindDN ?? node.parameters.user ?? "", first)) ||
    getCredential("LDAP_BIND_DN") ||
    "";
  const bindPassword =
    String(resolve(node.parameters.bindPassword ?? node.parameters.password ?? "", first)) ||
    getCredential("LDAP_PASSWORD") ||
    "";
  const baseDn = String(
    resolve(node.parameters.baseDn ?? "", first) ||
      getCredential("LDAP_BASE_DN") ||
      "",
  );
  requireCred(baseDn, "LDAP baseDn");
  const op = String(node.parameters.operation ?? "search");
  const filter = String(
    resolve(node.parameters.filter ?? "(objectClass=*)", first),
  );

  const { Client: LdapClient } = await import("ldapts");
  const client = new LdapClient({ url });
  try {
    if (bindDN) {
      await client.bind(bindDN, bindPassword);
    }

    if (op === "search") {
      const { searchEntries } = await client.search(baseDn, {
        scope: (node.parameters.scope as "sub" | "base" | "one") ?? "sub",
        filter,
        attributes: (node.parameters.attributes as string[]) ?? undefined,
      });
      return {
        items: searchEntries.map((e) =>
          makeItem(e as unknown as Record<string, unknown>),
        ),
      };
    }

    if (op === "compare") {
      const dn = String(resolve(node.parameters.dn ?? "", first));
      const attribute = String(resolve(node.parameters.attribute ?? "", first));
      const value = String(resolve(node.parameters.value ?? "", first));
      requireCred(dn, "LDAP compare dn");
      const matched = await client.compare(dn, attribute, value);
      return { items: [makeItem({ dn, attribute, matched })] };
    }

    throw new Error(
      `LDAP operation "${op}" not implemented yet (supported: search, compare)`,
    );
  } finally {
    await client.unbind().catch(() => undefined);
  }
};

export const debugHelperNode: NodeHandler = async ({ node, items }) => {
  const category = String(node.parameters.category ?? "throwError");
  if (category === "doNothing") return { items };
  if (category === "throwError") {
    throw new Error(String(node.parameters.message ?? "Node has thrown an error"));
  }
  const count = Math.max(1, Math.min(100, Number(node.parameters.count ?? 5)));
  const out: WorkflowItem[] = [];
  for (let i = 0; i < count; i++) {
    out.push(
      makeItem({
        index: i,
        id: randomBytes(8).toString("hex"),
        email: `user${i}@example.com`,
        uuid: `${randomBytes(4).toString("hex")}-${randomBytes(2).toString("hex")}-${randomBytes(2).toString("hex")}-${randomBytes(2).toString("hex")}-${randomBytes(6).toString("hex")}`,
      }),
    );
  }
  return { items: out };
};

export const evaluationNode: NodeHandler = async ({ node, items }) => {
  const metric = String(node.parameters.metric ?? "accuracy");
  const value = Number(node.parameters.value ?? 1);
  const base = items[0] ? itemJson(items[0]) : {};
  return {
    items: [
      makeItem({
        ...base,
        evaluation: {
          metric,
          value,
          expected: node.parameters.expected ?? null,
          actual: node.parameters.actual ?? null,
          itemCount: items.length,
          at: new Date().toISOString(),
        },
      }),
    ],
  };
};
