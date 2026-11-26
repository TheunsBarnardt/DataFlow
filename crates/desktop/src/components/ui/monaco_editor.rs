//! Monaco editor host for Dioxus desktop (webview + CDN loader).
//! Parity with web `CodeEditor.tsx`: $vars completions, inline decorations, drop-into-editor.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::panels::{normalize_drag_expr, DF_DRAG_MIME};
use crate::format_code::{format_code, CodeLanguage};

fn monaco_lang(language: CodeLanguage, hint: Option<&str>) -> &'static str {
    if let Some(h) = hint {
        match h.to_ascii_lowercase().as_str() {
            "typescript" | "ts" => return "typescript",
            "javascript" | "js" => return "javascript",
            "json" => return "json",
            "sql" | "tsql" => return "sql",
            _ => {}
        }
    }
    match language {
        CodeLanguage::Sql => "sql",
        CodeLanguage::Json => "json",
        CodeLanguage::Js => "javascript",
        CodeLanguage::Text => "plaintext",
    }
}

#[component]
pub fn MonacoEditor(
    /// Reactive so prop updates (e.g. SQL loaded after mount) re-sync into the editor.
    value: ReadSignal<String>,
    on_change: EventHandler<String>,
    #[props(default)] language: CodeLanguage,
    #[props(default = 140)] height_px: u32,
    #[props(default)] read_only: bool,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] language_hint: Option<String>,
    #[props(default)] id: Option<String>,
    #[props(default = true)] show_format: bool,
    #[props(default)] expr_bus: Option<Signal<Option<String>>>,
    #[props(default)] active_field: Option<Signal<Option<String>>>,
) -> Element {
    // Prefer the caller id (node-scoped) so switching NDV nodes remounts cleanly.
    let editor_id = id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("bf-monaco-{}", Uuid::new_v4()));
    let lang = monaco_lang(language, language_hint.as_deref());
    let host_id = editor_id.clone();
    let can_format = show_format && language.supports_format() && !read_only;
    let mut drag_over = use_signal(|| false);
    // Create the Monaco host once per component instance. Re-running create was
    // disposing the editor and echoing an empty model back into parameters.
    let mut bootstrapped = use_signal(|| false);
    // Show a plain textarea until Monaco paints (or permanently if CDN/host fails).
    let mut fallback = use_signal(|| true);
    let mut monaco_ready = use_signal(|| false);
    let mut fallback_text = use_signal(|| value());
    // Ignore empty model echoes briefly after create — they wipe Query/SQL params.
    let mut ignore_empty_echo = use_signal(|| false);

    let field_id_effect = editor_id.clone();
    use_effect(move || {
        let Some(mut bus) = expr_bus else {
            return;
        };
        let Some(expr) = bus() else {
            return;
        };
        let active = active_field.and_then(|a| a());
        if active.as_deref() != Some(field_id_effect.as_str()) {
            return;
        }
        let normalized = normalize_drag_expr(&expr);
        bus.set(None);
        on_change.call(normalized);
    });

    use_effect({
        let editor_id = editor_id.clone();
        move || {
            if bootstrapped() {
                return;
            }
            bootstrapped.set(true);
            ignore_empty_echo.set(true);
            let editor_id = editor_id.clone();
            // Capture at create time; reactive sync below keeps the model current.
            let initial = value();
            let lang = lang.to_string();
            let read_only = read_only;
            // Safety: always lift the empty-echo guard even if ready never arrives.
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                ignore_empty_echo.set(false);
            });
            spawn(async move {
                // Expose API base so Monaco loads from the local engine, not the CDN.
                let _ = document::eval(&format!(
                    "window.__bfApiBase = {};",
                    serde_json::to_string(&crate::api::base_url()).unwrap_or_else(|_| "\"\"".into()),
                ));
                let script = format!(
                    r#"
(async () => {{
  const id = {id};
  const lang = {lang};
  const readOnly = {read_only};
  const initial = {initial};
  const DRAG_MIME = {drag_mime};

  function waitForEl(elId, attempts) {{
    return new Promise((resolve) => {{
      let n = 0;
      const tick = () => {{
        const el = document.getElementById(elId);
        if (el) {{ resolve(el); return; }}
        n += 1;
        if (n >= attempts) {{ resolve(null); return; }}
        setTimeout(tick, 50);
      }};
      tick();
    }});
  }}

  function ensureLoader() {{
    return new Promise((resolve, reject) => {{
      if (window.monaco) {{ resolve(window.monaco); return; }}
      // Prefer local engine copy (CDN is often blocked in the desktop webview).
      const base = (window.__bfApiBase || '').replace(/\/$/, '');
      const localVs = base + '/vendor/monaco/vs';
      const cdnVs = 'https://cdn.jsdelivr.net/npm/monaco-editor@0.52.2/min/vs';
      const tryLoad = (vsPath, onFail) => {{
        const existing = document.querySelector('script[data-bf-monaco-loader]');
        const start = () => {{
          try {{
            window.require.config({{ paths: {{ vs: vsPath }} }});
            window.require(['vs/editor/editor.main'], () => {{
              if (window.monaco) resolve(window.monaco);
              else onFail(new Error('monaco missing after load'));
            }}, onFail);
          }} catch (e) {{ onFail(e); }}
        }};
        if (window.require && window.require.config) {{ start(); return; }}
        if (existing) {{
          existing.addEventListener('load', start);
          existing.addEventListener('error', () => onFail(new Error('loader script failed')));
          return;
        }}
        const s = document.createElement('script');
        s.src = vsPath + '/loader.js';
        s.dataset.bfMonacoLoader = '1';
        s.onload = start;
        s.onerror = () => onFail(new Error('failed to fetch ' + s.src));
        document.head.appendChild(s);
      }};
      tryLoad(localVs, () => tryLoad(cdnVs, reject));
    }});
  }}

  function keyQueryFromToken(query) {{
    const q = (query || '').trim().toLowerCase();
    if (!q) return '';
    if (q.startsWith('vars.')) return q.slice(5);
    if ('vars'.startsWith(q)) return '';
    return q;
  }}

  function filterVariables(items, query) {{
    const keyQuery = keyQueryFromToken(query);
    if (!keyQuery) return items;
    return items.filter((v) =>
      (v.key || '').toLowerCase().includes(keyQuery) ||
      (v.description || '').toLowerCase().includes(keyQuery)
    );
  }}

  function applyVarDecorations(editor, monaco, decorationIds) {{
    const model = editor.getModel();
    if (!model) return decorationIds;
    const matches = model.findMatches(
      '\\$vars\\.[A-Za-z_][A-Za-z0-9_]*',
      false, true, false, null, false
    );
    const next = matches.map((m) => ({{
      range: m.range,
      options: {{
        inlineClassName: 'bf-monaco-resolvable',
        stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
      }},
    }}));
    return editor.deltaDecorations(decorationIds, next);
  }}

  const el = await waitForEl(id, 40);
  if (!el) {{
    dioxus.send('__bf_monaco_missing_host__');
    return;
  }}

  let monaco;
  try {{
    monaco = await ensureLoader();
  }} catch (err) {{
    dioxus.send('__bf_monaco_load_failed__');
    return;
  }}
  window.__bfMonaco = window.__bfMonaco || {{}};

  if (window.__bfMonaco[id]?.editor) {{
    try {{ window.__bfMonaco[id].cleanupDrag?.(); }} catch (_) {{}}
    try {{ window.__bfMonaco[id].editor.dispose(); }} catch (_) {{}}
    delete window.__bfMonaco[id];
  }}

  // Desktop webview: avoid cross-origin worker failures that leave the host blank.
  const vsBase = ((window.__bfApiBase || '').replace(/\/$/, '') + '/vendor/monaco/vs');
  window.MonacoEnvironment = {{
    getWorkerUrl: function (_moduleId, _label) {{
      const body =
        "self.MonacoEnvironment = {{ baseUrl: '" + vsBase + "/' }};" +
        "importScripts('" + vsBase + "/base/worker/workerMain.js');";
      return URL.createObjectURL(new Blob([body], {{ type: 'text/javascript' }}));
    }},
  }};

  const dark = document.documentElement.classList.contains('dark');
  monaco.editor.setTheme(dark ? 'vs-dark' : 'vs');
  const editor = monaco.editor.create(el, {{
    value: initial,
    language: lang,
    readOnly,
    automaticLayout: true,
    minimap: {{ enabled: false }},
    fontSize: 12,
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
    lineNumbers: 'on',
    lineNumbersMinChars: 3,
    glyphMargin: false,
    folding: false,
    scrollBeyondLastLine: false,
    wordWrap: 'on',
    tabSize: 2,
    renderLineHighlight: 'line',
    scrollbar: {{ verticalScrollbarSize: 8, horizontalScrollbarSize: 8 }},
    padding: {{ top: 8, bottom: 8 }},
    overviewRulerLanes: 0,
    hideCursorInOverviewRuler: true,
    overviewRulerBorder: false,
    suggestOnTriggerCharacters: true,
    quickSuggestions: {{ other: true, comments: false, strings: true }},
  }});

  let decorationIds = [];
  decorationIds = applyVarDecorations(editor, monaco, decorationIds);

  window.__bfMonaco[id] = {{
    editor,
    monaco,
    decorationIds: () => decorationIds,
    suppress: false,
    lastSent: initial,
  }};

  // $vars completion once per language
  window.__bfMonacoCompletions = window.__bfMonacoCompletions || new Set();
  if (!window.__bfMonacoCompletions.has(lang)) {{
    window.__bfMonacoCompletions.add(lang);
    monaco.languages.registerCompletionItemProvider(lang, {{
      triggerCharacters: ['$', '.'],
      provideCompletionItems(model, position) {{
        const line = model.getValueInRange({{
          startLineNumber: position.lineNumber,
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
        }});
        const m = line.match(/\$[A-Za-z0-9_.]*$/);
        if (!m) return {{ suggestions: [] }};
        const token = m[0];
        const query = token.slice(1);
        const vars = window.__bfVariables || [];
        const filtered = filterVariables(vars, query);
        const startColumn = position.column - token.length;
        const range = {{
          startLineNumber: position.lineNumber,
          startColumn,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
        }};
        const suggestions = filtered.slice(0, 40).map((v, i) => ({{
          label: `$vars.${{v.key}}`,
          kind: monaco.languages.CompletionItemKind.Variable,
          insertText: `$vars.${{v.key}}`,
          detail: v.value || '',
          documentation: v.description || undefined,
          sortText: String(i + 1).padStart(4, '0'),
          range,
        }}));
        if (!query || 'vars'.startsWith(query.toLowerCase())) {{
          suggestions.unshift({{
            label: '$vars.',
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: '$vars.',
            detail: 'Insert variable prefix',
            sortText: '0000',
            range,
          }});
        }}
        return {{ suggestions }};
      }},
    }});
  }}

  const themeObs = new MutationObserver(() => {{
    monaco.editor.setTheme(document.documentElement.classList.contains('dark') ? 'vs-dark' : 'vs');
  }});
  themeObs.observe(document.documentElement, {{ attributes: true, attributeFilter: ['class'] }});

  editor.onDidChangeModelContent(() => {{
    decorationIds = applyVarDecorations(editor, monaco, decorationIds);
    const slot = window.__bfMonaco[id];
    if (!slot || slot.suppress) return;
    const v = editor.getValue();
    // Skip echo of the bootstrap / external setValue — prevents wiping Query etc.
    if (v === slot.lastSent) return;
    slot.lastSent = v;
    dioxus.send(v);
  }});

  // Drop expressions from NDV data pane into the editor
  const dom = editor.getDomNode();
  if (dom) {{
    const host = dom.closest('[data-bf-monaco]') || el.parentElement || el;
    const onDragOver = (e) => {{
      if (readOnly) return;
      const types = e.dataTransfer?.types || [];
      if ([...types].includes(DRAG_MIME) || [...types].includes('text/plain')) {{
        e.preventDefault();
        e.dataTransfer.dropEffect = 'copy';
        host.classList.add('ring-2', 'ring-[var(--brand)]');
      }}
    }};
    const onDragLeave = () => host.classList.remove('ring-2', 'ring-[var(--brand)]');
    const onDrop = (e) => {{
      if (readOnly) return;
      e.preventDefault();
      host.classList.remove('ring-2', 'ring-[var(--brand)]');
      const raw =
        e.dataTransfer.getData(DRAG_MIME) ||
        e.dataTransfer.getData('text/plain') ||
        '';
      if (!raw.trim()) return;
      let expr = raw.trim();
      if (!expr.startsWith('={{') && !expr.startsWith('{{')) {{
        expr = '={{ ' + expr + ' }}';
      }} else if (expr.startsWith('{{') && !expr.startsWith('={{')) {{
        expr = '=' + expr;
      }}
      const sel = editor.getSelection();
      if (sel) {{
        editor.executeEdits('bf-drop', [{{
          range: sel,
          text: expr,
          forceMoveMarkers: true,
        }}]);
      }} else {{
        editor.setValue(expr);
      }}
      editor.focus();
    }};
    host.addEventListener('dragover', onDragOver);
    host.addEventListener('dragleave', onDragLeave);
    host.addEventListener('drop', onDrop);
    window.__bfMonaco[id].cleanupDrag = () => {{
      host.removeEventListener('dragover', onDragOver);
      host.removeEventListener('dragleave', onDragLeave);
      host.removeEventListener('drop', onDrop);
    }};
  }}

  dioxus.send('__bf_monaco_ready__');

  while (true) {{
    const msg = await dioxus.recv();
    if (msg && typeof msg === 'object') {{
      if (typeof msg.value === 'string' && msg.value !== editor.getValue()) {{
        const slot = window.__bfMonaco[id];
        if (slot) {{
          slot.suppress = true;
          slot.lastSent = msg.value;
        }}
        try {{
          const pos = editor.getPosition();
          editor.setValue(msg.value);
          if (pos) editor.setPosition(pos);
          decorationIds = applyVarDecorations(editor, monaco, decorationIds);
        }} finally {{
          if (slot) slot.suppress = false;
        }}
      }}
      if (typeof msg.themeDark === 'boolean') {{
        monaco.editor.setTheme(msg.themeDark ? 'vs-dark' : 'vs');
      }}
      if (Array.isArray(msg.variables)) {{
        window.__bfVariables = msg.variables;
        decorationIds = applyVarDecorations(editor, monaco, decorationIds);
      }}
      if (typeof msg.insert === 'string' && msg.insert) {{
        const slot = window.__bfMonaco[id];
        if (slot) slot.lastSent = msg.insert;
        const sel = editor.getSelection();
        if (sel) {{
          editor.executeEdits('bf-insert', [{{
            range: sel,
            text: msg.insert,
            forceMoveMarkers: true,
          }}]);
        }} else {{
          editor.setValue(msg.insert);
        }}
        decorationIds = applyVarDecorations(editor, monaco, decorationIds);
      }}
    }}
  }}
  try {{ themeObs.disconnect(); }} catch (_) {{}}
  try {{
    window.__bfMonaco[id]?.cleanupDrag?.();
    editor.dispose();
    delete window.__bfMonaco[id];
  }} catch (_) {{}}
}})()
"#,
                    id = serde_json::to_string(&editor_id).unwrap_or_else(|_| "\"\"".into()),
                    lang = serde_json::to_string(&lang).unwrap_or_else(|_| "\"plaintext\"".into()),
                    read_only = if read_only { "true" } else { "false" },
                    initial = serde_json::to_string(&initial).unwrap_or_else(|_| "\"\"".into()),
                    drag_mime = serde_json::to_string(DF_DRAG_MIME).unwrap_or_else(|_| "\"\"".into()),
                );

                let mut eval = document::eval(&script);
                // Seed variables list
                if let Ok(v) = crate::api::get_json("/api/variables").await {
                    let vars: Vec<serde_json::Value> = v
                        .get("variables")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|item| {
                            serde_json::json!({
                                "key": item.get("key").and_then(|x| x.as_str()).unwrap_or(""),
                                "value": item.get("value").and_then(|x| x.as_str()).unwrap_or(""),
                                "description": item.get("description").and_then(|x| x.as_str()).unwrap_or(""),
                            })
                        })
                        .collect();
                    let _ = eval.send(serde_json::json!({ "variables": vars }));
                }

                while let Ok(next) = eval.recv::<String>().await {
                    if next == "__bf_monaco_missing_host__" || next == "__bf_monaco_load_failed__" {
                        let current = value();
                        fallback_text.set(current);
                        fallback.set(true);
                        monaco_ready.set(false);
                        ignore_empty_echo.set(false);
                        break;
                    }
                    if next == "__bf_monaco_ready__" {
                        // Re-push latest prop in case bootstrap captured a stale empty string.
                        let current = value();
                        let _ = eval.send(serde_json::json!({ "value": current }));
                        monaco_ready.set(true);
                        fallback.set(false);
                        ignore_empty_echo.set(false);
                        continue;
                    }
                    // Drop empty echoes only during bootstrap — otherwise intentional clears break.
                    if next.is_empty() && ignore_empty_echo() {
                        let current = value();
                        if !current.is_empty() {
                            let _ = eval.send(serde_json::json!({ "value": current }));
                        }
                        continue;
                    }
                    on_change.call(next);
                }
            });
        }
    });

    // Push external value changes into Monaco (ReadSignal makes this reactive).
    use_effect({
        let editor_id = editor_id.clone();
        move || {
            let next = value();
            fallback_text.set(next.clone());
            if !monaco_ready() {
                return;
            }
            let editor_id = editor_id.clone();
            spawn(async move {
                let _ = document::eval(&format!(
                    r#"(function(){{
  const slot = window.__bfMonaco && window.__bfMonaco[{id}];
  if (!slot || !slot.editor) return;
  const next = {value};
  if (slot.editor.getValue() === next) {{
    slot.lastSent = next;
    return;
  }}
  slot.suppress = true;
  slot.lastSent = next;
  try {{
    const pos = slot.editor.getPosition();
    slot.editor.setValue(next);
    if (pos) slot.editor.setPosition(pos);
  }} finally {{
    slot.suppress = false;
  }}
}})()"#,
                    id = serde_json::to_string(&editor_id).unwrap_or_else(|_| "\"\"".into()),
                    value = serde_json::to_string(&next).unwrap_or_else(|_| "\"\"".into()),
                ));
            });
        }
    });

    let eid_drop = editor_id.clone();
    use_drop(move || {
        let _ = document::eval(&format!(
            r#"(function(){{
  const id = {id};
  const slot = window.__bfMonaco && window.__bfMonaco[id];
  if (slot?.cleanupDrag) try {{ slot.cleanupDrag(); }} catch (_) {{}}
  if (slot?.editor) {{
    try {{ slot.editor.dispose(); }} catch (_) {{}}
    delete window.__bfMonaco[id];
  }}
}})()"#,
            id = serde_json::to_string(&eid_drop).unwrap_or_else(|_| "\"\"".into()),
        ));
    });

    let height = format!("height:{height_px}px;min-height:{height_px}px");
    let ph = placeholder.unwrap_or_default();
    let host_class = if drag_over() {
        "w-full overflow-hidden rounded-md border border-input bg-background ring-2 ring-[var(--brand)]"
    } else {
        "w-full overflow-hidden rounded-md border border-input bg-background focus-within:ring-2 focus-within:ring-ring"
    };

    rsx! {
        div { class: "flex w-full flex-col gap-1",
            if can_format {
                div { class: "flex justify-end",
                    button {
                        r#type: "button",
                        class: "rounded px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground hover:bg-muted",
                        title: "Format",
                        onclick: move |_| {
                            let result = format_code(&value(), language);
                            if result.ok {
                                on_change.call(result.text);
                            }
                        },
                        "Format"
                    }
                }
            }
            // Keep the Monaco host mounted (needed for create) and overlay a textarea
            // until syntax highlighting is ready — Query never goes blank.
            div {
                class: "relative w-full",
                style: "{height}",
                if fallback() || !monaco_ready() {
                    textarea {
                        class: "absolute inset-0 z-10 w-full resize-none rounded-md border border-input bg-background px-3 py-2 font-mono text-xs text-foreground",
                        value: "{fallback_text}",
                        placeholder: "{ph}",
                        readonly: read_only,
                        spellcheck: false,
                        oninput: move |e| {
                            let next = e.value();
                            fallback_text.set(next.clone());
                            on_change.call(next);
                        },
                    }
                }
                div {
                    class: "{host_class}",
                    "data-bf-monaco": "1",
                    title: if ph.is_empty() { String::new() } else { ph.clone() },
                    onfocusin: {
                        let editor_id = editor_id.clone();
                        move |_| {
                            if let Some(mut active) = active_field {
                                active.set(Some(editor_id.clone()));
                            }
                        }
                    },
                    ondragover: move |e| {
                        if read_only { return; }
                        e.prevent_default();
                        drag_over.set(true);
                    },
                    ondragleave: move |_| drag_over.set(false),
                    ondrop: move |e| {
                        if read_only { return; }
                        e.prevent_default();
                        drag_over.set(false);
                        let dt = e.data().data_transfer();
                        let expr = dt
                            .get_data(DF_DRAG_MIME)
                            .or_else(|| dt.get_as_text());
                        let Some(expr) = expr.filter(|s| !s.trim().is_empty()) else {
                            return;
                        };
                        let normalized = normalize_drag_expr(&expr);
                        let editor_id = editor_id.clone();
                        let insert = normalized.clone();
                        spawn(async move {
                            let _ = document::eval(&format!(
                                r#"(function(){{
  const slot = window.__bfMonaco && window.__bfMonaco[{id}];
  if (!slot || !slot.editor) return;
  const text = {text};
  const sel = slot.editor.getSelection();
  if (sel) {{
    slot.editor.executeEdits('bf-drop', [{{ range: sel, text, forceMoveMarkers: true }}]);
  }} else {{
    slot.editor.setValue(text);
  }}
  slot.editor.focus();
}})()"#,
                                id = serde_json::to_string(&editor_id).unwrap_or_else(|_| "\"\"".into()),
                                text = serde_json::to_string(&insert).unwrap_or_else(|_| "\"\"".into()),
                            ));
                        });
                        on_change.call(normalized);
                    },
                    div {
                        id: "{host_id}",
                        class: "w-full",
                        style: "{height}",
                    }
                }
            }
        }
    }
}
