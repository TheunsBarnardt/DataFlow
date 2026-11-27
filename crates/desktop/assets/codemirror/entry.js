import { EditorView, basicSetup } from "codemirror";
import { EditorState, Compartment } from "@codemirror/state";
import { keymap, placeholder as cmPlaceholder } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { sql } from "@codemirror/lang-sql";
import { json } from "@codemirror/lang-json";
import { javascript } from "@codemirror/lang-javascript";
import { oneDark } from "@codemirror/theme-one-dark";
import {
  autocompletion,
  completionKeymap,
} from "@codemirror/autocomplete";
import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";

function langExt(lang) {
  switch ((lang || "").toLowerCase()) {
    case "sql":
    case "tsql":
      return sql();
    case "json":
      return json();
    case "javascript":
    case "js":
    case "typescript":
    case "ts":
      return javascript();
    default:
      return [];
  }
}

function isDark() {
  if (document.documentElement.classList.contains("dark")) return true;
  if (document.body && document.body.classList.contains("dark")) return true;
  try {
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  } catch (_) {
    return true;
  }
}

function varCompletions(context) {
  const before = context.matchBefore(/\$[A-Za-z0-9_.]*/);
  if (!before || (before.from === before.to && !context.explicit)) return null;
  const token = before.text;
  const query = token.slice(1).toLowerCase();
  const vars = window.__bfVariables || [];
  const options = [];
  if (!query || "vars".startsWith(query)) {
    options.push({ label: "$vars.", type: "keyword", apply: "$vars." });
  }
  const keyQuery = query.startsWith("vars.") ? query.slice(5) : query === "vars" ? "" : query;
  for (const v of vars) {
    const key = (v.key || "").toLowerCase();
    if (keyQuery && !key.includes(keyQuery)) continue;
    options.push({
      label: `$vars.${v.key}`,
      type: "variable",
      detail: v.value || "",
      info: v.description || undefined,
      apply: `$vars.${v.key}`,
    });
    if (options.length >= 40) break;
  }
  return { from: before.from, options };
}

function baseTheme() {
  return EditorView.theme({
    "&": {
      height: "100%",
      fontSize: "12px",
      color: "var(--foreground, #e8e8e8)",
      backgroundColor: "transparent",
    },
    ".cm-scroller": {
      overflow: "auto",
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
    },
    ".cm-content": {
      padding: "8px 0",
      caretColor: "var(--foreground, #e8e8e8)",
      color: "var(--foreground, #e8e8e8)",
      minHeight: "100%",
    },
    ".cm-line": { color: "inherit" },
    ".cm-gutters": {
      backgroundColor: "transparent",
      color: "var(--muted-foreground, #9ca3af)",
      border: "none",
    },
    ".cm-activeLineGutter": { backgroundColor: "transparent" },
    "&.cm-focused": { outline: "none" },
  });
}

function create(host, opts) {
  const langComp = new Compartment();
  const roComp = new Compartment();
  const themeComp = new Compartment();
  const phComp = new Compartment();
  const dark = isDark();
  const exts = [
    basicSetup,
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap, ...completionKeymap, indentWithTab]),
    langComp.of(langExt(opts.language)),
    roComp.of(EditorState.readOnly.of(!!opts.readOnly)),
    themeComp.of(dark ? oneDark : syntaxHighlighting(defaultHighlightStyle, { fallback: true })),
    baseTheme(),
    phComp.of(opts.placeholder ? cmPlaceholder(opts.placeholder) : []),
    autocompletion({ override: [varCompletions] }),
    EditorView.updateListener.of((update) => {
      if (!update.docChanged) return;
      const slot = window.__bfCm && window.__bfCm[opts.id];
      if (!slot || slot.suppress) return;
      const v = update.state.doc.toString();
      if (v === slot.lastSent) return;
      slot.lastSent = v;
      if (typeof opts.onChange === "function") opts.onChange(v);
    }),
  ];

  const view = new EditorView({
    parent: host,
    state: EditorState.create({
      doc: opts.value || "",
      extensions: exts,
    }),
  });

  const slot = {
    view,
    langComp,
    roComp,
    themeComp,
    phComp,
    lastSent: opts.value || "",
    suppress: false,
    setValue(next) {
      const text = next == null ? "" : String(next);
      if (view.state.doc.toString() === text) {
        slot.lastSent = text;
        return;
      }
      slot.suppress = true;
      slot.lastSent = text;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: text },
      });
      slot.suppress = false;
    },
    setLanguage(lang) {
      view.dispatch({ effects: langComp.reconfigure(langExt(lang)) });
    },
    setReadOnly(ro) {
      view.dispatch({ effects: roComp.reconfigure(EditorState.readOnly.of(!!ro)) });
    },
    setDark(isDarkMode) {
      view.dispatch({
        effects: themeComp.reconfigure(
          isDarkMode
            ? oneDark
            : syntaxHighlighting(defaultHighlightStyle, { fallback: true })
        ),
      });
    },
    insertAtCursor(text) {
      const sel = view.state.selection.main;
      const insert = text == null ? "" : String(text);
      slot.suppress = true;
      view.dispatch({
        changes: { from: sel.from, to: sel.to, insert },
        selection: { anchor: sel.from + insert.length },
      });
      slot.lastSent = view.state.doc.toString();
      slot.suppress = false;
      view.focus();
      // Notify Rust even though suppress blocked the updateListener.
      if (typeof opts.onChange === "function") opts.onChange(slot.lastSent);
      return slot.lastSent;
    },
    focus() {
      view.focus();
    },
    destroy() {
      try {
        slot._themeObs && slot._themeObs.disconnect();
      } catch (_) {}
      view.destroy();
    },
  };

  window.__bfCm = window.__bfCm || {};
  window.__bfCm[opts.id] = slot;

  const themeObs = new MutationObserver(() => {
    slot.setDark(isDark());
  });
  themeObs.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
  if (document.body) {
    themeObs.observe(document.body, { attributes: true, attributeFilter: ["class"] });
  }
  slot._themeObs = themeObs;

  // Ensure layout after attach (webview often measures 0 first).
  requestAnimationFrame(() => {
    try {
      view.requestMeasure();
    } catch (_) {}
  });

  return slot;
}

window.bfCodeMirror = { create, langExt };

// Dioxus desktop serializes drag events and stubs DataTransfer.set_data, so HTML5
// drag payloads never stick unless we set them on the real browser event.
(function installExprDnD() {
  if (window.__bfExprDnD && window.__bfExprDnD.bound) return;
  window.__bfExprDnD = {
    mime: "application/x-dataflow-expr",
    payload: null,
    bound: false,
    normalize(expr) {
      const t = String(expr || "").trim();
      if (!t) return "";
      if (t.startsWith("={{")) return t;
      if (t.startsWith("{{")) return `=${t}`;
      return `={{ ${t} }}`;
    },
    consume() {
      const v = this.payload;
      this.payload = null;
      return v;
    },
    bind() {
      if (this.bound) return;
      this.bound = true;
      const self = this;
      document.addEventListener(
        "dragstart",
        (e) => {
          const el =
            e.target && e.target.closest && e.target.closest("[data-bf-expr]");
          if (!el || !e.dataTransfer) return;
          const expr = el.getAttribute("data-bf-expr") || "";
          if (!expr) return;
          self.payload = expr;
          try {
            e.dataTransfer.setData(self.mime, expr);
            e.dataTransfer.setData("text/plain", expr);
            e.dataTransfer.effectAllowed = "copy";
          } catch (_) {}
        },
        true
      );
    },
  };
  window.__bfExprDnD.bind();
})();
