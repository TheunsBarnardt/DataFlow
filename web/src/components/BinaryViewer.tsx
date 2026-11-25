import { Download } from "lucide-react";
import { Button } from "@/components/ui/button";

export type BinaryPayload = {
  key: string;
  data: string;
  mimeType: string;
  fileName?: string;
};

function asRecord(v: unknown): Record<string, unknown> | null {
  if (!v || typeof v !== "object" || Array.isArray(v)) return null;
  return v as Record<string, unknown>;
}

function isBinaryShape(
  v: unknown,
): v is { data: string; mimeType?: string; fileName?: string } {
  const o = asRecord(v);
  return Boolean(o && typeof o.data === "string" && (o.data as string).length > 0);
}

/** Collect binary payloads from n8n-style items / {data,mimeType} blobs. */
export function extractBinaryPayloads(value: unknown): BinaryPayload[] {
  const out: BinaryPayload[] = [];
  const seen = new Set<string>();

  function push(key: string, data: string, mimeType?: string, fileName?: string) {
    const id = `${key}:${data.slice(0, 32)}`;
    if (seen.has(id)) return;
    seen.add(id);
    out.push({
      key,
      data,
      mimeType: mimeType || "application/octet-stream",
      fileName,
    });
  }

  function walk(node: unknown, path = "data") {
    if (node == null) return;
    if (Array.isArray(node)) {
      node.forEach((item, i) => walk(item, `${path}[${i}]`));
      return;
    }
    const obj = asRecord(node);
    if (!obj) return;
    const entries = Object.entries(obj);
    const binaryBag = asRecord(obj.binary);

    if (
      isBinaryShape(obj) &&
      ("mimeType" in obj || path.toLowerCase().includes("binary"))
    ) {
      push(
        path,
        String(obj.data),
        typeof obj.mimeType === "string" ? obj.mimeType : undefined,
        typeof obj.fileName === "string" ? obj.fileName : undefined,
      );
    }

    if (binaryBag) {
      for (const [k, v] of Object.entries(binaryBag)) {
        if (isBinaryShape(v)) {
          push(k, v.data, v.mimeType, v.fileName);
        } else if (typeof v === "string") {
          push(k, v);
        } else {
          walk(v, `${path}.binary.${k}`);
        }
      }
    }

    for (const [k, v] of entries) {
      if (k === "binary") continue;
      if (k.toLowerCase().includes("binary") && isBinaryShape(v)) {
        push(k, v.data, v.mimeType);
      } else if (typeof v === "object" && v !== null) {
        walk(v, `${path}.${k}`);
      }
    }
  }

  walk(value);
  return out;
}

export function hasBinaryData(value: unknown): boolean {
  return extractBinaryPayloads(value).length > 0;
}

function looksLikeBase64(s: string): boolean {
  if (s.length < 8) return false;
  return /^[A-Za-z0-9+/=\r\n]+$/.test(s.slice(0, 200));
}

function toBlob(payload: BinaryPayload): Blob {
  const raw = payload.data;
  if (looksLikeBase64(raw)) {
    try {
      const bin = atob(raw.replace(/\s/g, ""));
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      return new Blob([bytes], { type: payload.mimeType });
    } catch {
      /* fall through */
    }
  }
  return new Blob([raw], { type: payload.mimeType || "text/plain" });
}

function previewText(payload: BinaryPayload): string | null {
  const mime = payload.mimeType.toLowerCase();
  const textish =
    mime.startsWith("text/") ||
    mime.includes("json") ||
    mime.includes("xml") ||
    mime.includes("javascript") ||
    mime === "application/octet-stream";
  if (!textish && !looksLikeBase64(payload.data)) return null;

  let decoded = payload.data;
  if (looksLikeBase64(payload.data)) {
    try {
      decoded = atob(payload.data.replace(/\s/g, ""));
    } catch {
      decoded = payload.data;
    }
  }
  if (decoded.length > 4000) return `${decoded.slice(0, 4000)}\n…`;
  // Avoid dumping huge binary garbage into the UI
  if (/[\x00-\x08\x0e-\x1f]/.test(decoded.slice(0, 200))) return null;
  return decoded;
}

function download(payload: BinaryPayload) {
  const blob = toBlob(payload);
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download =
    payload.fileName ||
    `${payload.key.replace(/[^\w.-]+/g, "_") || "binary"}.${
      payload.mimeType.includes("json")
        ? "json"
        : payload.mimeType.includes("pdf")
          ? "pdf"
          : payload.mimeType.startsWith("text/")
            ? "txt"
            : "bin"
    }`;
  a.click();
  URL.revokeObjectURL(url);
}

export function BinaryViewer({ data }: { data: unknown }) {
  const payloads = extractBinaryPayloads(data);
  if (!payloads.length) return null;

  return (
    <div className="flex flex-col gap-2 rounded-md border border-amber-500/30 bg-amber-500/5 p-2">
      <div className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
        Binary data
      </div>
      {payloads.map((p) => {
        const text = previewText(p);
        return (
          <div
            key={`${p.key}-${p.mimeType}`}
            className="flex flex-col gap-1.5 rounded border bg-background/80 p-2"
          >
            <div className="flex items-center gap-2">
              <code className="truncate text-[11px] font-medium">{p.key}</code>
              <span className="text-[10px] text-muted-foreground">
                {p.mimeType}
              </span>
              <Button
                type="button"
                size="xs"
                variant="outline"
                className="ml-auto h-6 gap-1 px-1.5 text-[10px]"
                onClick={() => download(p)}
              >
                <Download className="size-3" />
                Download
              </Button>
            </div>
            {text ? (
              <pre className="max-h-32 overflow-auto rounded bg-muted/60 p-1.5 text-[10px] leading-snug break-all whitespace-pre-wrap">
                {text}
              </pre>
            ) : (
              <p className="text-[10px] text-muted-foreground">
                Binary payload — use Download to save.
              </p>
            )}
          </div>
        );
      })}
    </div>
  );
}
