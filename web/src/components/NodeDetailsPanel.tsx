import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  Pin,
  Play,
  Trash2,
  X,
  Zap,
} from "lucide-react";
import type { NodeSettings, WorkflowNode } from "@dataflow/shared";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { FormSelect } from "@/components/ui/form-select";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api } from "@/lib/api";
import type { NodeCatalogItem } from "@/components/NodesPanel";
import type { NodeUpdatePatch } from "@/components/FlowCanvas";
import {
  NodeParameterForm,
  sanitizeParameters,
} from "@/components/NodeParameterForm";
import { defaultParameters, getNodeSchema } from "@/lib/nodeSchemas";
import {
  WebhookListenPanel,
  type WebhookSample,
} from "@/components/WebhookListenPanel";
import { NdvDataPane } from "@/components/NdvDataPane";
import { getNodeColor, getNodeIcon } from "@/lib/nodeIcons";
import { sampleFromApiEndpointParameters } from "@/lib/jsonSchemaSample";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

const DEFAULT_SETTINGS: NodeSettings = {
  alwaysOutputData: false,
  executeOnce: false,
  retryOnFail: false,
  maxTries: 3,
  waitBetweenTries: 1000,
  onError: "stopWorkflow",
  notes: "",
  displayNoteInFlow: false,
  disabled: false,
};

const AUTO_APPLY_MS = 400;

function asItems(data: unknown): unknown[] {
  if (data == null) return [];
  return Array.isArray(data) ? data : [data];
}

function itemJson(item: unknown): unknown {
  if (item && typeof item === "object" && "json" in (item as object)) {
    return (item as { json: unknown }).json;
  }
  return item;
}

export function NodeDetailsPanel({
  selectedNode,
  catalogHint,
  workflowId,
  workflowEnabled,
  onClose,
  onUpdateNode,
  onDeleteNode,
  sampleOutput,
  onSampleOutput,
  onRunRecorded,
  inputSample,
  readOnly = false,
}: {
  selectedNode: WorkflowNode | null;
  catalogHint?: NodeCatalogItem | null;
  workflowId?: string;
  workflowEnabled?: boolean;
  onClose: () => void;
  onUpdateNode?: (id: string, patch: NodeUpdatePatch) => void | Promise<void>;
  onDeleteNode?: (id: string) => void;
  sampleOutput?: unknown;
  onSampleOutput?: (nodeId: string, sample: unknown) => void;
  /** Called after Execute step so Runs / Executions panels can refresh. */
  onRunRecorded?: () => void;
  /** Upstream / execute input sample for the Input column. */
  inputSample?: unknown;
  /** Executions view — browse I/O without editing. */
  readOnly?: boolean;
}) {
  const [catalog, setCatalog] = useState<NodeCatalogItem[]>([]);
  const [name, setName] = useState("");
  const [parameters, setParameters] = useState<Record<string, unknown>>({});
  const [settings, setSettings] = useState<NodeSettings>(DEFAULT_SETTINGS);
  const [dirty, setDirty] = useState(false);
  const [centerTab, setCenterTab] = useState("parameters");
  const [executing, setExecuting] = useState(false);
  const [outputPreview, setOutputPreview] = useState<unknown>(null);
  const [webhookSample, setWebhookSample] = useState<WebhookSample | null>(
    null,
  );
  const [inputIndex, setInputIndex] = useState(0);
  const [outputIndex, setOutputIndex] = useState(0);
  const applyingRef = useRef(false);
  const latestRef = useRef({
    name,
    parameters,
    settings,
    selectedNode,
    schema: null as ReturnType<typeof getNodeSchema> | null,
  });

  useEffect(() => {
    void api<{ nodes: NodeCatalogItem[] }>("/api/nodes").then((r) =>
      setCatalog(r.nodes),
    );
  }, []);

  useEffect(() => {
    if (!selectedNode) {
      setName("");
      setParameters({});
      setSettings(DEFAULT_SETTINGS);
      setDirty(false);
      setOutputPreview(null);
      setWebhookSample(null);
      setInputIndex(0);
      setOutputIndex(0);
      return;
    }
    setName(selectedNode.name);
    const defaults = defaultParameters(selectedNode.type);
    setParameters({ ...defaults, ...(selectedNode.parameters ?? {}) });
    setSettings({ ...DEFAULT_SETTINGS, ...(selectedNode.settings ?? {}) });
    setDirty(false);
    setOutputPreview(sampleOutput ?? null);
    setWebhookSample(null);
    setCenterTab("parameters");
    setInputIndex(0);
    setOutputIndex(0);
  }, [selectedNode, sampleOutput]);

  const schema = useMemo(
    () => (selectedNode ? getNodeSchema(selectedNode.type) : null),
    [selectedNode],
  );

  latestRef.current = {
    name,
    parameters,
    settings,
    selectedNode,
    schema,
  };

  const focus =
    catalogHint ??
    (selectedNode
      ? (catalog.find((c) => c.type === selectedNode.type) ?? null)
      : null);

  const isWebhook = selectedNode?.type === "webhook";
  const NodeIcon = selectedNode ? getNodeIcon(selectedNode.type) : null;
  const nodeColor = selectedNode
    ? getNodeColor(selectedNode.type)
    : "var(--brand)";

  /** Live request-shape sample while editing apiEndpoint schemas. */
  const apiSchemaSample = useMemo(() => {
    if (selectedNode?.type !== "apiEndpoint") return undefined;
    return sampleFromApiEndpointParameters(parameters);
  }, [selectedNode?.type, parameters]);

  const effectiveInputSample = inputSample ?? apiSchemaSample;
  const effectiveOutputSample = outputPreview ?? apiSchemaSample;

  const inputItems = useMemo(
    () => asItems(effectiveInputSample),
    [effectiveInputSample],
  );
  const outputItems = useMemo(
    () => asItems(effectiveOutputSample),
    [effectiveOutputSample],
  );

  const safeInputIndex = Math.min(
    inputIndex,
    Math.max(0, inputItems.length - 1),
  );
  const safeOutputIndex = Math.min(
    outputIndex,
    Math.max(0, outputItems.length - 1),
  );

  const currentInputItem =
    inputItems.length > 0 ? inputItems[safeInputIndex] : null;
  const currentOutputItem =
    outputItems.length > 0 ? outputItems[safeOutputIndex] : null;

  const applyChanges = useCallback(
    async (opts?: { silent?: boolean }) => {
      const {
        name: n,
        parameters: p,
        settings: s,
        selectedNode: node,
        schema: sch,
      } = latestRef.current;
      if (!node || !onUpdateNode || !sch) return false;
      const trimmed = n.trim();
      if (!trimmed) {
        if (!opts?.silent) toast.error("Node name is required");
        return false;
      }
      const result = sanitizeParameters(sch, p);
      if (!result.ok) {
        if (!opts?.silent) toast.error(result.error);
        return false;
      }
      if (applyingRef.current) return false;
      applyingRef.current = true;
      try {
        await onUpdateNode(node.id, {
          name: trimmed,
          parameters: result.parameters,
          settings: s,
        });
        setDirty(false);
        if (!opts?.silent) toast.success("Node updated");
        return true;
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to save node");
        return false;
      } finally {
        applyingRef.current = false;
      }
    },
    [onUpdateNode],
  );

  // Auto-apply when dirty (debounced).
  useEffect(() => {
    if (!dirty || !selectedNode || !onUpdateNode || !schema) return;
    const t = window.setTimeout(() => {
      void applyChanges({ silent: true });
    }, AUTO_APPLY_MS);
    return () => window.clearTimeout(t);
  }, [
    dirty,
    name,
    parameters,
    settings,
    selectedNode,
    onUpdateNode,
    schema,
    applyChanges,
  ]);

  function handleDelete() {
    if (!selectedNode || !onDeleteNode) return;
    onDeleteNode(selectedNode.id);
  }

  async function executeStep() {
    if (!selectedNode || !workflowId || !schema) return;

    let parametersToRun = parameters;
    if (dirty && onUpdateNode) {
      const trimmed = name.trim();
      if (!trimmed) {
        toast.error("Node name is required");
        return;
      }
      const result = sanitizeParameters(schema, parameters);
      if (!result.ok) {
        toast.error(result.error);
        return;
      }
      parametersToRun = result.parameters;
      await onUpdateNode(selectedNode.id, {
        name: trimmed,
        parameters: result.parameters,
        settings,
      });
      setDirty(false);
    }

    setExecuting(true);
    try {
      const res = await api<{
        output: unknown;
        input: unknown;
        error?: string;
      }>(`/api/workflows/${workflowId}/execute-node`, {
        method: "POST",
        body: JSON.stringify({
          nodeId: selectedNode.id,
          // Don't feed this node's previous output back as input (that made
          // MSSQL Input look like FAQ rows after Execute step).
          input: effectiveInputSample ?? webhookSample ?? undefined,
          parameters: parametersToRun,
        }),
      });
      if (res.error) {
        toast.error(res.error);
        setOutputPreview({ error: res.error });
      } else {
        setOutputPreview(res.output);
        setOutputIndex(0);
        onSampleOutput?.(selectedNode.id, res.output);
        toast.success("Step executed");
      }
      onRunRecorded?.();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Execute failed");
    } finally {
      setExecuting(false);
    }
  }

  function patchSetting<K extends keyof NodeSettings>(
    key: K,
    value: NodeSettings[K],
  ) {
    setSettings((prev) => ({ ...prev, [key]: value }));
    setDirty(true);
  }

  function pinOutput() {
    if (!selectedNode || effectiveOutputSample == null) return;
    onSampleOutput?.(selectedNode.id, effectiveOutputSample);
    toast.success("Output pinned");
  }

  if (!selectedNode) return null;

  return (
    <div className="absolute inset-0 z-40" data-ndv>
      <button
        type="button"
        aria-label="Close node editor"
        className="absolute inset-0 bg-black/45"
        onClick={onClose}
      />
      <aside
        role="dialog"
        aria-modal="true"
        aria-label={name || selectedNode.name}
        className="absolute inset-4 flex flex-col overflow-hidden rounded-xl border bg-background shadow-2xl sm:inset-5 md:inset-6"
      >
      {/* Header */}
      <div
        className="flex shrink-0 items-center gap-3 border-b px-4 py-2.5"
        style={{ boxShadow: "inset 3px 0 0 var(--brand)" }}
      >
          <button
            type="button"
            onClick={onClose}
            className="rounded-md p-1.5 text-muted-foreground hover:bg-muted"
            title="Back"
          >
            <ArrowLeft className="h-4 w-4" />
          </button>

          <div
            className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-white"
            style={{ background: nodeColor }}
          >
            {NodeIcon ? (
              <NodeIcon className="h-4 w-4" strokeWidth={2} />
            ) : (
              <span className="text-xs font-bold">
                {(focus?.label ?? selectedNode.name).slice(0, 2).toUpperCase()}
              </span>
            )}
          </div>

          <div className="min-w-0 flex-1">
            <Input
              id="node-name"
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setDirty(true);
              }}
              className="h-8 border-transparent bg-transparent px-1 text-sm font-semibold shadow-none focus-visible:border-input focus-visible:bg-background"
              placeholder="Node name"
            />
            <div className="flex items-center gap-1.5 px-1 text-[11px] text-muted-foreground">
              <span className="truncate">{selectedNode.type}</span>
              {focus?.category === "trigger" ? (
                <Zap className="h-3 w-3 shrink-0 text-amber-500" />
              ) : null}
              {focus ? (
                <Badge className="h-4 px-1 text-[9px]">Installed</Badge>
              ) : null}
              {dirty ? (
                <span className="text-[10px] text-amber-600 dark:text-amber-400">
                  Unsaved
                </span>
              ) : null}
            </div>
          </div>

          <div className="flex shrink-0 items-center gap-1.5">
            {workflowId && !isWebhook && !readOnly ? (
              <Button
                size="sm"
                variant="outline"
                disabled={executing}
                onClick={() => void executeStep()}
              >
                <Play className="h-3.5 w-3.5" />
                {executing ? "Running…" : "Execute step"}
              </Button>
            ) : null}
            {!readOnly ? (
              <Button
                size="sm"
                variant="outline"
                className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                disabled={!onDeleteNode}
                onClick={handleDelete}
              >
                <Trash2 className="h-3.5 w-3.5" />
                Delete
              </Button>
            ) : (
              <Badge variant="secondary" className="text-[10px]">
                Execution view
              </Badge>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="px-2"
              onClick={onClose}
              title="Close"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* 3 columns */}
        <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[minmax(220px,1fr)_minmax(320px,1.4fr)_minmax(220px,1fr)]">
          {/* Input */}
          <section className="flex min-h-0 flex-col border-r">
            <div className="flex shrink-0 items-center justify-between border-b px-3 py-2">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Input
              </h3>
              {inputItems.length > 1 ? (
                <ItemPager
                  index={safeInputIndex}
                  total={inputItems.length}
                  onChange={setInputIndex}
                />
              ) : inputItems.length === 1 ? (
                <span className="text-[10px] text-muted-foreground">
                  1 item
                </span>
              ) : null}
            </div>
            <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden p-3">
              <NdvDataPane
                title="Input data"
                data={effectiveInputSample ?? currentInputItem}
                emptyHint={
                  selectedNode?.type === "apiEndpoint"
                    ? "Add Query Params / Body (Key + Value) like Postman to preview $json.query / $json.body here."
                    : "No input data yet. Execute an upstream node, run the workflow, or open a run in Executions."
                }
              />
            </div>
          </section>

          {/* Parameters + Settings */}
          <section className="flex min-h-0 flex-col border-r">
            <Tabs
              value={centerTab}
              onValueChange={setCenterTab}
              className="flex min-h-0 flex-1 flex-col"
            >
              <div className="shrink-0 border-b px-3 pt-2">
                <TabsList className="w-full">
                  <TabsTrigger value="parameters" className="flex-1">
                    Parameters
                  </TabsTrigger>
                  <TabsTrigger value="settings" className="flex-1">
                    Settings
                  </TabsTrigger>
                </TabsList>
              </div>
              <ScrollArea className="min-h-0 flex-1">
                <div className="p-3">
                  {schema ? (
                    <>
                      <TabsContent
                        value="parameters"
                        className="mt-0 flex flex-col gap-3"
                      >
                        {isWebhook && workflowId ? (
                          <WebhookListenPanel
                            workflowId={workflowId}
                            path={String(parameters.path ?? "")}
                            method={String(parameters.method ?? "POST")}
                            workflowEnabled={Boolean(workflowEnabled)}
                            sample={webhookSample}
                            onSample={(s) => {
                              setWebhookSample(s);
                              if (s) {
                                const mapped = {
                                  headers: s.headers,
                                  query: s.query,
                                  body: s.body,
                                  method: s.method,
                                };
                                setOutputPreview([mapped]);
                                onSampleOutput?.(selectedNode.id, [mapped]);
                              }
                            }}
                          />
                        ) : null}

                        {readOnly ? (
                          <p className="rounded-md border bg-muted/30 px-3 py-2 text-xs text-muted-foreground">
                            Viewing execution data (read-only). Switch to Editor
                            to change parameters.
                          </p>
                        ) : null}
                        <fieldset disabled={readOnly} className="min-w-0">
                          <NodeParameterForm
                            schema={schema}
                            values={parameters}
                            onChange={(next) => {
                              setParameters(next);
                              setDirty(true);
                            }}
                          />
                        </fieldset>
                      </TabsContent>

                      <TabsContent
                        value="settings"
                        className="mt-0 flex flex-col gap-3"
                      >
                        <fieldset
                          disabled={readOnly}
                          className="flex flex-col gap-3"
                        >
                          <div className="flex items-center gap-2">
                            <Switch
                              id="settings-alwaysOutputData"
                              checked={Boolean(settings.alwaysOutputData)}
                              onCheckedChange={(checked) =>
                                patchSetting("alwaysOutputData", checked)
                              }
                            />
                            <Label htmlFor="settings-alwaysOutputData">
                              Always Output Data
                            </Label>
                          </div>
                          <div className="flex items-center gap-2">
                            <Switch
                              id="settings-executeOnce"
                              checked={Boolean(settings.executeOnce)}
                              onCheckedChange={(checked) =>
                                patchSetting("executeOnce", checked)
                              }
                            />
                            <Label htmlFor="settings-executeOnce">
                              Execute Once
                            </Label>
                          </div>
                          <div className="flex items-center gap-2">
                            <Switch
                              id="settings-retryOnFail"
                              checked={Boolean(settings.retryOnFail)}
                              onCheckedChange={(checked) =>
                                patchSetting("retryOnFail", checked)
                              }
                            />
                            <Label htmlFor="settings-retryOnFail">
                              Retry On Fail
                            </Label>
                          </div>
                          {settings.retryOnFail ? (
                            <>
                              <div className="flex flex-col gap-1.5">
                                <span className="text-[11px] font-medium text-muted-foreground">
                                  Max Tries
                                </span>
                                <Input
                                  type="number"
                                  min={1}
                                  className="h-9"
                                  value={Number(settings.maxTries ?? 3)}
                                  onChange={(e) =>
                                    patchSetting(
                                      "maxTries",
                                      Math.max(1, Number(e.target.value) || 3),
                                    )
                                  }
                                />
                              </div>
                              <div className="flex flex-col gap-1.5">
                                <span className="text-[11px] font-medium text-muted-foreground">
                                  Wait Between Tries (ms)
                                </span>
                                <Input
                                  type="number"
                                  min={0}
                                  className="h-9"
                                  value={Number(settings.waitBetweenTries ?? 1000)}
                                  onChange={(e) =>
                                    patchSetting(
                                      "waitBetweenTries",
                                      Math.max(0, Number(e.target.value) || 0),
                                    )
                                  }
                                />
                              </div>
                            </>
                          ) : null}
                          <div className="flex items-center gap-2">
                            <Switch
                              id="settings-disabled"
                              checked={Boolean(settings.disabled)}
                              onCheckedChange={(checked) =>
                                patchSetting("disabled", checked)
                              }
                            />
                            <Label htmlFor="settings-disabled">
                              Deactivated
                            </Label>
                          </div>
                          <div className="flex flex-col gap-1.5">
                            <span className="text-[11px] font-medium text-muted-foreground">
                              On Error
                            </span>
                            <FormSelect
                              value={settings.onError ?? "stopWorkflow"}
                              options={[
                                { value: "stopWorkflow", label: "Stop Workflow" },
                                { value: "continue", label: "Continue" },
                                {
                                  value: "continueErrorOutput",
                                  label: "Continue (using error output)",
                                },
                              ]}
                              onChange={(next) =>
                                patchSetting(
                                  "onError",
                                  next as NodeSettings["onError"],
                                )
                              }
                            />
                          </div>
                          <div className="flex flex-col gap-1.5">
                            <span className="text-[11px] font-medium text-muted-foreground">
                              Notes
                            </span>
                            <Textarea
                              className="min-h-[72px] text-xs"
                              value={settings.notes ?? ""}
                              onChange={(e) =>
                                patchSetting("notes", e.target.value)
                              }
                            />
                          </div>
                          <div className="flex items-center gap-2">
                            <Switch
                              id="settings-displayNoteInFlow"
                              checked={Boolean(settings.displayNoteInFlow)}
                              onCheckedChange={(checked) =>
                                patchSetting("displayNoteInFlow", checked)
                              }
                            />
                            <Label htmlFor="settings-displayNoteInFlow">
                              Display note in flow
                            </Label>
                          </div>
                          <div className="rounded-md border bg-muted/20 px-2.5 py-1.5 text-[11px] text-muted-foreground">
                            Id{" "}
                            <code className="ml-1 text-foreground">
                              {selectedNode.id}
                            </code>
                          </div>
                        </fieldset>
                      </TabsContent>
                    </>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      Unknown node type.
                    </p>
                  )}
                </div>
              </ScrollArea>
            </Tabs>
          </section>

          {/* Output */}
          <section className="flex min-h-0 flex-col">
            <div className="flex shrink-0 items-center justify-between gap-2 border-b px-3 py-2">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Output
              </h3>
              <div className="flex items-center gap-1.5">
                {outputItems.length > 1 ? (
                  <ItemPager
                    index={safeOutputIndex}
                    total={outputItems.length}
                    onChange={setOutputIndex}
                  />
                ) : outputItems.length === 1 ? (
                  <span className="text-[10px] text-muted-foreground">
                    1 item
                  </span>
                ) : null}
                <Button
                  size="xs"
                  variant="outline"
                  disabled={effectiveOutputSample == null}
                  onClick={pinOutput}
                  title="Pin output as sample"
                >
                  <Pin className="h-3 w-3" />
                  Pin
                </Button>
              </div>
            </div>
            <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden p-3">
              <NdvDataPane
                title="Output data"
                data={effectiveOutputSample ?? currentOutputItem}
                emptyHint={
                  isWebhook
                    ? "Listen for a test event to map incoming fields."
                    : selectedNode?.type === "apiEndpoint"
                      ? "Add Query Params / Body values to preview request output for mapping."
                      : "Execute step or open a run in Executions."
                }
                onPinData={readOnly ? undefined : pinOutput}
              />
            </div>
          </section>
        </div>
      </aside>
    </div>
  );
}

function ItemPager({
  index,
  total,
  onChange,
}: {
  index: number;
  total: number;
  onChange: (i: number) => void;
}) {
  return (
    <div className="flex items-center gap-0.5 text-[11px] text-muted-foreground">
      <button
        type="button"
        className="rounded p-0.5 hover:bg-muted disabled:opacity-40"
        disabled={index <= 0}
        onClick={() => onChange(index - 1)}
      >
        <ChevronLeft className="h-3.5 w-3.5" />
      </button>
      <span className="min-w-[3.5rem] text-center tabular-nums">
        {index + 1} / {total}
      </span>
      <button
        type="button"
        className="rounded p-0.5 hover:bg-muted disabled:opacity-40"
        disabled={index >= total - 1}
        onClick={() => onChange(index + 1)}
      >
        <ChevronRight className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
