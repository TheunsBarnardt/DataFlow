import { useCallback, useEffect, useRef } from "react";
import Editor, { type OnMount } from "@monaco-editor/react";
import type { editor, IDisposable, languages } from "monaco-editor";
import { useVariables, type VariableItem } from "@/hooks/useVariables";
import { cn } from "@/lib/utils";

const TOKEN_BEFORE_RE = /\$[A-Za-z0-9_.]*$/;

export type CodeEditorLanguage =
  | "sql"
  | "json"
  | "javascript"
  | "typescript"
  | "plaintext";

type CodeEditorProps = {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  language?: CodeEditorLanguage;
  height?: number | string;
  className?: string;
  placeholder?: string;
  readOnly?: boolean;
};

function monacoLanguage(lang: CodeEditorLanguage): string {
  if (lang === "javascript") return "javascript";
  if (lang === "typescript") return "typescript";
  if (lang === "json") return "json";
  if (lang === "sql") return "sql";
  return "plaintext";
}

function isDarkTheme(): boolean {
  return document.documentElement.classList.contains("dark");
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

/** Shared so every Monaco instance sees the latest variable list. */
const variablesBridge: { current: VariableItem[] } = { current: [] };

const completionRegistered = new Set<string>();
const completionDisposables: IDisposable[] = [];

function ensureVarsCompletion(
  monaco: typeof import("monaco-editor"),
  languageId: string,
) {
  if (completionRegistered.has(languageId)) return;
  completionRegistered.add(languageId);
  const disposable = monaco.languages.registerCompletionItemProvider(
    languageId,
    {
      triggerCharacters: ["$", "."],
      provideCompletionItems(
        model,
        position,
      ): languages.ProviderResult<languages.CompletionList> {
        const line = model.getValueInRange({
          startLineNumber: position.lineNumber,
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
        });
        const m = line.match(TOKEN_BEFORE_RE);
        if (!m) return { suggestions: [] };

        const token = m[0];
        const query = token.slice(1);
        const filtered = filterVariables(variablesBridge.current, query);
        const startColumn = position.column - token.length;

        const suggestions: languages.CompletionItem[] = filtered.map(
          (item, i) => ({
            label: `$vars.${item.key}`,
            kind: monaco.languages.CompletionItemKind.Variable,
            insertText: `$vars.${item.key}`,
            detail: item.value,
            documentation: item.description || undefined,
            sortText: String(i + 1).padStart(4, "0"),
            range: {
              startLineNumber: position.lineNumber,
              startColumn,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
          }),
        );

        if (!query || "vars".startsWith(query.toLowerCase())) {
          suggestions.unshift({
            label: "$vars.",
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: "$vars.",
            detail: "Insert variable prefix",
            sortText: "0000",
            range: {
              startLineNumber: position.lineNumber,
              startColumn,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
          });
        }

        return { suggestions };
      },
    },
  );
  completionDisposables.push(disposable);
}

function applyVarDecorations(
  ed: editor.IStandaloneCodeEditor,
  monaco: typeof import("monaco-editor"),
  decorationIds: string[],
): string[] {
  const model = ed.getModel();
  if (!model) return decorationIds;
  const matches = model.findMatches(
    "\\$vars\\.[A-Za-z_][A-Za-z0-9_]*",
    false,
    true,
    false,
    null,
    false,
  );
  const next = matches.map((m) => ({
    range: m.range,
    options: {
      inlineClassName: "bf-monaco-resolvable",
      stickiness:
        monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
    },
  }));
  return ed.deltaDecorations(decorationIds, next);
}

export function CodeEditor({
  id,
  value,
  onChange,
  language = "plaintext",
  height = 160,
  className,
  placeholder,
  readOnly = false,
}: CodeEditorProps) {
  const { variables } = useVariables();
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<typeof import("monaco-editor") | null>(null);
  const decorationsRef = useRef<string[]>([]);

  useEffect(() => {
    variablesBridge.current = variables;
  }, [variables]);

  const refreshDecorations = useCallback(() => {
    const ed = editorRef.current;
    const monaco = monacoRef.current;
    if (!ed || !monaco) return;
    decorationsRef.current = applyVarDecorations(
      ed,
      monaco,
      decorationsRef.current,
    );
  }, []);

  useEffect(() => {
    refreshDecorations();
  }, [value, refreshDecorations]);

  useEffect(() => {
    const sync = () => {
      const monaco = monacoRef.current;
      if (!monaco) return;
      monaco.editor.setTheme(isDarkTheme() ? "vs-dark" : "vs");
    };
    sync();
    const obs = new MutationObserver(sync);
    obs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => obs.disconnect();
  }, []);

  const handleMount: OnMount = (ed, monaco) => {
    editorRef.current = ed;
    monacoRef.current = monaco;
    monaco.editor.setTheme(isDarkTheme() ? "vs-dark" : "vs");
    ensureVarsCompletion(monaco, monacoLanguage(language));

    decorationsRef.current = applyVarDecorations(
      ed,
      monaco,
      decorationsRef.current,
    );

    ed.onDidChangeModelContent(() => {
      decorationsRef.current = applyVarDecorations(
        ed,
        monaco,
        decorationsRef.current,
      );
    });

    if (id) {
      ed.getContainerDomNode().id = id;
    }
  };

  return (
    <div
      className={cn(
        "overflow-hidden rounded-md border bg-background",
        "focus-within:ring-2 focus-within:ring-ring",
        className,
      )}
    >
      <Editor
        height={height}
        language={monacoLanguage(language)}
        value={value}
        theme={isDarkTheme() ? "vs-dark" : "vs"}
        onMount={handleMount}
        onChange={(next) => onChange(next ?? "")}
        loading={
          <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
            Loading editor…
          </div>
        }
        options={{
          readOnly,
          minimap: { enabled: false },
          fontSize: 12,
          fontFamily:
            'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
          lineNumbers: "on",
          lineNumbersMinChars: 3,
          glyphMargin: false,
          folding: false,
          scrollBeyondLastLine: false,
          wordWrap: "on",
          automaticLayout: true,
          tabSize: 2,
          renderLineHighlight: "line",
          scrollbar: {
            verticalScrollbarSize: 8,
            horizontalScrollbarSize: 8,
          },
          padding: { top: 8, bottom: 8 },
          overviewRulerLanes: 0,
          hideCursorInOverviewRuler: true,
          overviewRulerBorder: false,
          ariaLabel: placeholder || "Code editor",
          suggestOnTriggerCharacters: true,
          quickSuggestions: {
            other: true,
            comments: false,
            strings: true,
          },
        }}
      />
    </div>
  );
}
