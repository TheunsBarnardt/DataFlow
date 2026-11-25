import type { WorkflowItem } from "./items.js";

/** Read base64 binary property from an item (supports flat or nested n8n shapes). */
export function getBinaryBuffer(
  item: WorkflowItem,
  property = "data",
): Buffer | null {
  const bin = item.binary?.[property] ?? item.binary?.data;
  if (bin == null) return null;
  if (typeof bin === "string") return Buffer.from(bin, "base64");
  if (typeof bin === "object" && bin && "data" in bin) {
    const data = (bin as { data: unknown }).data;
    if (typeof data === "string") return Buffer.from(data, "base64");
    if (Buffer.isBuffer(data)) return data;
  }
  return null;
}

export function setBinaryBuffer(
  item: WorkflowItem,
  buf: Buffer,
  opts?: { property?: string; mimeType?: string; fileName?: string },
): WorkflowItem {
  const property = opts?.property ?? "data";
  const mimeType = opts?.mimeType ?? "application/octet-stream";
  return {
    ...item,
    binary: {
      ...(item.binary ?? {}),
      [property]: {
        data: buf.toString("base64"),
        mimeType,
        fileSize: buf.byteLength,
        fileName: opts?.fileName ?? property,
      },
    },
  };
}
