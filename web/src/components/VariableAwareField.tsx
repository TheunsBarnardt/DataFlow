import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useVariables, type VariableItem } from "@/hooks/useVariables";
import { cn } from "@/lib/utils";

/** Match `$…` token immediately before the caret (e.g. `$`, `$v`, `$vars.`, `$vars.SQL`). */
const TOKEN_RE = /\$[A-Za-z0-9_.]*$/;
const VARS_HIGHLIGHT_RE = /(\$vars\.[A-Za-z_][A-Za-z0-9_]*)/g;

type TokenMatch = {
  start: number;
  end: number;
  query: string;
};

function tokenAtCaret(value: string, caret: number): TokenMatch | null {
  const before = value.slice(0, caret);
  const m = before.match(TOKEN_RE);
  if (!m || m.index == null) return null;
  return { start: m.index, end: caret, query: m[0].slice(1) };
}

function keyQueryFromToken(query: string): string {
  const q = query.trim().toLowerCase();
  if (!q) return "";
  if (q.startsWith("vars.")) return q.slice(5);
  if ("vars".startsWith(q)) return "";
  return q;
}

function filterVariables(items: VariableItem[], query: string): VariableItem[] {
  const keyQuery = keyQueryFromToken(query);
  if (!keyQuery) return items;
  return items.filter(
    (v) =>
      v.key.toLowerCase().includes(keyQuery) ||
      (v.description ?? "").toLowerCase().includes(keyQuery),
  );
}

function insertToken(
  value: string,
  token: TokenMatch,
  key: string,
): { next: string; caret: number } {
  // Built-in expression snippets (not $vars keys)
  const builtins: Record<string, string> = {
    "json.": "$json.",
    json: "$json.",
    "input.item": "$input.item",
    input: "$input.item",
    "('Node')": "$('NodeName').item.",
    now: "$now",
    today: "$today",
  };
  const insert =
    builtins[key] ??
    (key.startsWith("vars.") || key.includes(".")
      ? key.startsWith("$")
        ? key
        : `$vars.${key.replace(/^vars\./, "")}`
      : `$vars.${key}`);
  const next = value.slice(0, token.start) + insert + value.slice(token.end);
  return { next, caret: token.start + insert.length };
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

const RESOLVABLE_HIGHLIGHT_RE =
  /(\{\{[\s\S]*?\}\}|\$vars\.[A-Za-z_][A-Za-z0-9_]*|\$json\.[A-Za-z0-9_.]+|\$input\.[A-Za-z0-9_.()]+|\$\('[^']+'\)|\$now|\$today)/g;

/** Highlight expressions / $vars / $json like n8n resolvable chips. */
function highlightHtml(value: string): string {
  if (!value) return "";
  const parts = value.split(RESOLVABLE_HIGHLIGHT_RE);
  return (
    parts
      .map((part) => {
        if (
          part.startsWith("$") ||
          part.startsWith("{{") ||
          part.startsWith("={{")
        ) {
          return `<span class="bf-resolvable">${escapeHtml(part)}</span>`;
        }
        return escapeHtml(part);
      })
      .join("")
      .replace(/\n$/, "\n ")
  );
}

type CommonProps = {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  spellCheck?: boolean;
};

export function VariableAwareInput(props: CommonProps) {
  return <VariableAwareBase as="input" {...props} />;
}

export function VariableAwareTextarea(props: CommonProps) {
  return <VariableAwareBase as="textarea" {...props} />;
}

function VariableAwareBase({
  as,
  id,
  value,
  onChange,
  placeholder,
  className,
  spellCheck,
}: CommonProps & { as: "input" | "textarea" }) {
  const { variables } = useVariables();
  const listId = useId();
  const wrapRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement | null>(null);
  const mirrorRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [token, setToken] = useState<TokenMatch | null>(null);
  const [active, setActive] = useState(0);
  const [menuPos, setMenuPos] = useState<{ top: number; left: number } | null>(
    null,
  );

  const filtered = useMemo(() => {
    if (!token) return [] as VariableItem[];
    const q = token.query.toLowerCase();
    const fromVars = filterVariables(variables, token.query);
    if (q.startsWith("vars")) return fromVars;

    const stub = (key: string, description: string): VariableItem =>
      ({
        _id: `builtin-${key}`,
        key,
        value: "",
        type: "string",
        description,
        updatedAt: "",
        createdAt: "",
      }) as VariableItem;

    const builtins = [
      stub("json", "$json — current item JSON"),
      stub("input", "$input.item — current input"),
      stub("('Node')", "$('NodeName') — other node output"),
      stub("now", "$now — current Date"),
      stub("today", "$today — start of today"),
    ].filter((b) => {
      if (!q) return true;
      return (
        b.key.toLowerCase().includes(q) ||
        (b.description ?? "").toLowerCase().includes(q)
      );
    });

    const merged = [...builtins];
    for (const v of fromVars) {
      if (!merged.some((m) => m.key === v.key)) merged.push(v);
    }
    return merged.slice(0, 20);
  }, [token, variables]);

  const hasExpr = /(\$vars\.|\$json|\$input|=\{\{)/.test(value);

  useEffect(() => {
    setActive(0);
  }, [token?.query, filtered.length]);

  const syncMirrorScroll = useCallback(() => {
    const el = inputRef.current;
    const mirror = mirrorRef.current;
    if (!el || !mirror) return;
    mirror.scrollTop = el.scrollTop;
    mirror.scrollLeft = el.scrollLeft;
  }, []);

  const syncFromCaret = useCallback((nextValue: string, caret: number) => {
    const t = tokenAtCaret(nextValue, caret);
    if (t) {
      setToken(t);
      setOpen(true);
    } else {
      setToken(null);
      setOpen(false);
      setMenuPos(null);
    }
  }, []);

  // Position menu near caret using a hidden mirror measurement
  useLayoutEffect(() => {
    if (!open || !token || !inputRef.current || !wrapRef.current) return;
    const el = inputRef.current;
    const coords = caretCoordinates(el, token.end);
    if (!coords) {
      setMenuPos({ top: el.offsetHeight + 4, left: 0 });
      return;
    }
    setMenuPos({
      top: coords.top + coords.height + 4 - el.scrollTop,
      left: Math.min(coords.left - el.scrollLeft, el.clientWidth - 200),
    });
  }, [open, token, value]);

  const apply = useCallback(
    (item: VariableItem) => {
      if (!token) return;
      const { next, caret } = insertToken(value, token, item.key);
      onChange(next);
      setOpen(false);
      setToken(null);
      setMenuPos(null);
      requestAnimationFrame(() => {
        const el = inputRef.current;
        if (!el) return;
        el.focus();
        el.setSelectionRange(caret, caret);
      });
    },
    [token, value, onChange],
  );

  function onKeyDown(e: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) {
    if (!open || !filtered.length) {
      if (e.key === "Escape") setOpen(false);
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => (i + 1) % filtered.length);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => (i - 1 + filtered.length) % filtered.length);
      return;
    }
    if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      apply(filtered[active] ?? filtered[0]!);
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      setOpen(false);
      setToken(null);
      setMenuPos(null);
    }
  }

  const showMenu = open && token != null;
  const sharedClass = cn(
    "relative z-[1] w-full rounded-md border bg-transparent px-3 text-sm outline-none",
    "focus-visible:ring-2 focus-visible:ring-ring",
    // Keep caret; hide ink so the mirror highlight shows through
    "text-transparent caret-foreground selection:bg-primary/30",
    hasExpr && "border-primary/50",
    as === "textarea" ? "min-h-[88px] py-2" : "h-9 py-1",
    className,
  );

  const mirrorStyle: CSSProperties = {
    whiteSpace: as === "textarea" ? "pre-wrap" : "pre",
    wordBreak: "break-word",
    overflow: "hidden",
  };

  return (
    <div ref={wrapRef} className="relative">
      {/* Highlight mirror (n8n-style resolvable chips) */}
      <div
        ref={mirrorRef}
        aria-hidden
        className={cn(
          "pointer-events-none absolute inset-0 z-0 overflow-hidden rounded-md border border-transparent px-3 text-sm text-foreground",
          as === "textarea" ? "min-h-[88px] py-2" : "flex h-9 items-center py-1",
          className?.includes("font-mono") && "font-mono",
          className?.includes("text-xs") && "text-xs",
        )}
        style={mirrorStyle}
        dangerouslySetInnerHTML={{
          __html:
            value.length > 0
              ? highlightHtml(value)
              : `<span class="bf-placeholder">${escapeHtml(placeholder ?? "")}</span>`,
        }}
      />

      {as === "textarea" ? (
        <Textarea
          id={id}
          ref={(node) => {
            inputRef.current = node;
          }}
          className={sharedClass}
          value={value}
          placeholder=""
          spellCheck={spellCheck}
          aria-autocomplete="list"
          aria-controls={showMenu ? listId : undefined}
          aria-expanded={showMenu}
          onChange={(e) => {
            const next = e.target.value;
            onChange(next);
            syncFromCaret(next, e.target.selectionStart ?? next.length);
            syncMirrorScroll();
          }}
          onScroll={syncMirrorScroll}
          onKeyDown={onKeyDown}
          onClick={(e) =>
            syncFromCaret(value, e.currentTarget.selectionStart ?? 0)
          }
          onBlur={() => {
            setTimeout(() => {
              setOpen(false);
              setMenuPos(null);
            }, 150);
          }}
        />
      ) : (
        <Input
          id={id}
          ref={(node) => {
            inputRef.current = node;
          }}
          className={sharedClass}
          value={value}
          placeholder=""
          spellCheck={spellCheck}
          aria-autocomplete="list"
          aria-controls={showMenu ? listId : undefined}
          aria-expanded={showMenu}
          onChange={(e) => {
            const next = e.target.value;
            onChange(next);
            syncFromCaret(next, e.target.selectionStart ?? next.length);
          }}
          onKeyDown={onKeyDown}
          onClick={(e) =>
            syncFromCaret(value, e.currentTarget.selectionStart ?? 0)
          }
          onBlur={() => {
            setTimeout(() => {
              setOpen(false);
              setMenuPos(null);
            }, 150);
          }}
        />
      )}

      {showMenu ? (
        <div
          id={listId}
          role="listbox"
          className={cn(
            "absolute z-50 min-w-[220px] max-w-full max-h-52 overflow-auto rounded-md border bg-popover p-1 shadow-lg",
            "text-popover-foreground",
          )}
          style={
            menuPos
              ? { top: menuPos.top, left: Math.max(0, menuPos.left) }
              : { top: "100%", left: 0, marginTop: 4 }
          }
        >
          <div className="px-2 py-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            Variables
          </div>
          {variables.length === 0 ? (
            <div className="px-2 py-1.5 text-xs text-muted-foreground">
              No variables yet — add some under Variables
            </div>
          ) : filtered.length === 0 ? (
            <div className="px-2 py-1.5 text-xs text-muted-foreground">
              No matching variables
            </div>
          ) : (
            filtered.map((item, i) => (
              <button
                key={item._id}
                type="button"
                role="option"
                tabIndex={-1}
                aria-selected={i === active}
                className={cn(
                  "flex w-full items-center justify-between gap-3 rounded-sm px-2 py-1.5 text-left text-xs",
                  i === active
                    ? "bg-accent text-accent-foreground"
                    : "hover:bg-muted",
                )}
                onMouseDown={(e) => {
                  e.preventDefault();
                  apply(item);
                }}
                onClick={(e) => {
                  e.preventDefault();
                  apply(item);
                }}
                onMouseEnter={() => setActive(i)}
              >
                <span className="font-mono font-medium text-primary">
                  $vars.{item.key}
                </span>
                <span className="max-w-[55%] truncate text-[10px] text-muted-foreground">
                  {item.value}
                </span>
              </button>
            ))
          )}
        </div>
      ) : null}
    </div>
  );
}

/**
 * Approximate caret pixel position inside an input/textarea
 * (enough to place the variable menu near the `$`).
 */
function caretCoordinates(
  el: HTMLInputElement | HTMLTextAreaElement,
  position: number,
): { top: number; left: number; height: number } | null {
  if (el instanceof HTMLInputElement) {
    // Single-line: approximate with a measuring span
    const div = document.createElement("div");
    const style = getComputedStyle(el);
    for (const prop of [
      "font",
      "fontSize",
      "fontFamily",
      "fontWeight",
      "letterSpacing",
      "textTransform",
      "paddingLeft",
      "paddingRight",
      "borderLeftWidth",
      "borderRightWidth",
      "boxSizing",
    ] as const) {
      div.style[prop] = style[prop];
    }
    div.style.position = "absolute";
    div.style.visibility = "hidden";
    div.style.whiteSpace = "pre";
    div.textContent = el.value.slice(0, position);
    document.body.appendChild(div);
    const left = div.offsetWidth;
    document.body.removeChild(div);
    return { top: 0, left: left + 12, height: el.offsetHeight };
  }

  const div = document.createElement("div");
  const style = getComputedStyle(el);
  const props = [
    "direction",
    "boxSizing",
    "width",
    "height",
    "overflowX",
    "overflowY",
    "borderTopWidth",
    "borderRightWidth",
    "borderBottomWidth",
    "borderLeftWidth",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "fontStyle",
    "fontVariant",
    "fontWeight",
    "fontStretch",
    "fontSize",
    "fontSizeAdjust",
    "lineHeight",
    "fontFamily",
    "textAlign",
    "textTransform",
    "textIndent",
    "textDecoration",
    "letterSpacing",
    "wordSpacing",
    "tabSize",
    "whiteSpace",
    "wordBreak",
    "wordWrap",
  ] as const;
  div.style.position = "absolute";
  div.style.visibility = "hidden";
  div.style.whiteSpace = "pre-wrap";
  div.style.wordWrap = "break-word";
  for (const prop of props) {
    div.style[prop] = style[prop];
  }
  div.style.width = `${el.offsetWidth}px`;
  div.textContent = el.value.slice(0, position);
  const span = document.createElement("span");
  span.textContent = el.value.slice(position) || ".";
  div.appendChild(span);
  document.body.appendChild(div);
  const top = span.offsetTop;
  const left = span.offsetLeft;
  const height = span.offsetHeight;
  document.body.removeChild(div);
  return { top, left, height };
}
