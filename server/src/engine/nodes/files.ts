import fs from "node:fs/promises";
import path from "node:path";
import { createReadStream, createWriteStream } from "node:fs";
import { pipeline } from "node:stream/promises";
import { createGzip, createGunzip } from "node:zlib";
import * as XLSX from "xlsx";
import { XMLParser, XMLBuilder } from "fast-xml-parser";
import type { NodeHandler } from "../types.js";
import { itemJson, makeItem, ensureItems } from "../items.js";

export const readFileNode: NodeHandler = async ({ node }) => {
  const filePath = String(node.parameters.path ?? "");
  const encoding = (node.parameters.encoding as BufferEncoding) ?? "utf8";
  const content = await fs.readFile(filePath, encoding);
  return { items: [{ path: filePath, content }] };
};

export const writeFileNode: NodeHandler = async ({ node, items }) => {
  const filePath = String(node.parameters.path ?? "");
  const content =
    node.parameters.content !== undefined
      ? String(node.parameters.content)
      : JSON.stringify(items, null, 2);
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, content, "utf8");
  return { items: [{ path: filePath, bytes: Buffer.byteLength(content) }] };
};

export const listFilesNode: NodeHandler = async ({ node }) => {
  const dir = String(node.parameters.path ?? ".");
  const entries = await fs.readdir(dir, { withFileTypes: true });
  return {
    items: entries.map((e) => ({
      name: e.name,
      path: path.join(dir, e.name),
      isDirectory: e.isDirectory(),
    })),
  };
};

export const moveFileNode: NodeHandler = async ({ node }) => {
  const from = String(node.parameters.from ?? "");
  const to = String(node.parameters.to ?? "");
  await fs.mkdir(path.dirname(to), { recursive: true });
  await fs.rename(from, to);
  return { items: [{ from, to }] };
};

export const copyFileNode: NodeHandler = async ({ node }) => {
  const from = String(node.parameters.from ?? "");
  const to = String(node.parameters.to ?? "");
  await fs.mkdir(path.dirname(to), { recursive: true });
  await fs.copyFile(from, to);
  return { items: [{ from, to }] };
};

export const deleteFileNode: NodeHandler = async ({ node }) => {
  const filePath = String(node.parameters.path ?? "");
  await fs.rm(filePath, { force: true, recursive: Boolean(node.parameters.recursive) });
  return { items: [{ path: filePath, deleted: true }] };
};

export const csvNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "parse");
  if (op === "parse") {
    const text =
      String(node.parameters.content ?? "") ||
      String(items[0] ? itemJson(items[0]).content ?? "" : "");
    const lines = text.trim().split(/\r?\n/);
    if (!lines.length) return { items: [] };
    const headers = lines[0]!.split(",").map((h) => h.trim());
    const rows = lines.slice(1).map((line) => {
      const cols = line.split(",");
      const row: Record<string, unknown> = {};
      headers.forEach((h, i) => {
        row[h] = cols[i]?.trim() ?? "";
      });
      return makeItem(row);
    });
    return { items: rows };
  }
  const first = items[0] ? itemJson(items[0]) : { value: "" };
  const headers = Object.keys(first);
  const lines = [
    headers.join(","),
    ...items.map((item) => {
      const json = itemJson(item);
      return headers.map((h) => String(json[h] ?? "")).join(",");
    }),
  ];
  return { items: [makeItem({ content: lines.join("\n"), format: "csv" })] };
};

export const jsonNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "parse");
  if (op === "parse") {
    const text =
      String(node.parameters.content ?? "") ||
      String(items[0] ? itemJson(items[0]).content ?? "[]" : "[]");
    const data = JSON.parse(text);
    return {
      items: Array.isArray(data)
        ? ensureItems(data)
        : [makeItem({ value: data })],
    };
  }
  return {
    items: [
      makeItem({
        content: JSON.stringify(
          items.map((i) => itemJson(i)),
          null,
          2,
        ),
        format: "json",
      }),
    ],
  };
};

export const xmlNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "wrap");
  const root = String(node.parameters.root ?? "items");
  if (op === "wrap" || op === "build") {
    const builder = new XMLBuilder({
      ignoreAttributes: false,
      format: true,
      suppressEmptyNode: true,
    });
    const payload =
      items.length === 1
        ? { [root]: itemJson(items[0]!) }
        : { [root]: { item: items.map((i) => itemJson(i)) } };
    const content = builder.build(payload);
    return {
      items: [makeItem({ content: String(content), format: "xml" })],
    };
  }

  // parse
  const text =
    String(node.parameters.content ?? "") ||
    String(items[0] ? itemJson(items[0]).content ?? "" : "");
  if (!text.trim()) {
    throw new Error("XML content required to parse");
  }
  const parser = new XMLParser({
    ignoreAttributes: false,
    attributeNamePrefix: "@_",
    trimValues: true,
  });
  const parsed = parser.parse(text) as unknown;
  if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
    const obj = parsed as Record<string, unknown>;
    const keys = Object.keys(obj);
    if (keys.length === 1) {
      const inner = obj[keys[0]!];
      if (Array.isArray(inner)) return { items: ensureItems(inner) };
      if (inner && typeof inner === "object") {
        const asObj = inner as Record<string, unknown>;
        if (Array.isArray(asObj.item)) return { items: ensureItems(asObj.item) };
        return { items: [makeItem(asObj)] };
      }
    }
    return { items: [makeItem(obj)] };
  }
  return { items: [makeItem({ value: parsed })] };
};

export const excelNode: NodeHandler = async ({ node, items }) => {
  const op = String(node.parameters.operation ?? "read");
  if (op === "read") {
    const filePath = String(node.parameters.path ?? "");
    const wb = XLSX.readFile(filePath);
    const sheet = wb.Sheets[wb.SheetNames[0]!];
    const rows = XLSX.utils.sheet_to_json<Record<string, unknown>>(sheet);
    return { items: ensureItems(rows) };
  }
  const filePath = String(node.parameters.path ?? "output.xlsx");
  const wb = XLSX.utils.book_new();
  const sheet = XLSX.utils.json_to_sheet(items.map((i) => itemJson(i)));
  XLSX.utils.book_append_sheet(wb, sheet, "Sheet1");
  XLSX.writeFile(wb, filePath);
  return { items: [makeItem({ path: filePath, rows: items.length })] };
};

export const zipNode: NodeHandler = async ({ node }) => {
  const op = String(node.parameters.operation ?? "zip");
  const from = String(node.parameters.from ?? "");
  if (!from) throw new Error("Zip: source path required");

  if (op === "gunzip" || op === "gzip") {
    const to = String(
      node.parameters.to ?? (op === "gunzip" ? from.replace(/\.gz$/i, "") : `${from}.gz`),
    );
    if (op === "gunzip") {
      await pipeline(createReadStream(from), createGunzip(), createWriteStream(to));
    } else {
      await pipeline(createReadStream(from), createGzip(), createWriteStream(to));
    }
    return { items: [makeItem({ from, to, operation: op })] };
  }

  if (op === "unzip") {
    const to = String(node.parameters.to ?? path.join(path.dirname(from), "unzipped"));
    await fs.mkdir(to, { recursive: true });
    const unzipper = await import("unzipper");
    await createReadStream(from)
      .pipe(unzipper.Extract({ path: to }))
      .promise();
    return { items: [makeItem({ from, to, operation: op })] };
  }

  // zip (create archive)
  const to = String(
    node.parameters.to ??
      (from.endsWith(".zip") ? from : `${from.replace(/\/$/, "")}.zip`),
  );
  await fs.mkdir(path.dirname(to), { recursive: true });
  const archiverImport = (await import("archiver")) as unknown as {
    default?: (format: string, opts?: { zlib?: { level?: number } }) => ArchiverLike;
    (format: string, opts?: { zlib?: { level?: number } }): ArchiverLike;
  };
  type ArchiverLike = {
    pipe: (s: NodeJS.WritableStream) => void;
    directory: (dir: string, dest: boolean | string) => void;
    file: (file: string, opts: { name: string }) => void;
    finalize: () => Promise<void>;
    pointer: () => number;
    on: (event: string, cb: (err: Error) => void) => void;
  };
  const createArchive = archiverImport.default ?? archiverImport;
  const output = createWriteStream(to);
  const archive = createArchive("zip", { zlib: { level: 9 } });
  const done = new Promise<void>((resolve, reject) => {
    output.on("close", () => resolve());
    archive.on("error", reject);
  });
  archive.pipe(output);
  const st = await fs.stat(from);
  if (st.isDirectory()) {
    archive.directory(from, false);
  } else {
    archive.file(from, { name: path.basename(from) });
  }
  await archive.finalize();
  await done;
  return { items: [makeItem({ from, to, operation: "zip", bytes: archive.pointer() })] };
};
