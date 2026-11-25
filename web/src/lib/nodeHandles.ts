/** Source handle ids for multi-output nodes (edge.sourceHandle). */
export function sourceHandlesForType(
  type: string,
  switchCases?: string[],
  opts?: { errorOutput?: boolean },
): string[] {
  let handles: string[];
  switch (type) {
    case "if":
      handles = ["true", "false"];
      break;
    case "filter":
    case "removeDuplicates":
      handles = ["kept", "discarded"];
      break;
    case "splitInBatches":
      handles = ["loop", "done"];
      break;
    case "switch": {
      const cases = (switchCases ?? []).filter(Boolean);
      // switchCases from FlowCanvas already includes fallback
      handles = cases.length ? [...new Set(cases)] : ["default"];
      break;
    }
    case "compareDatasets":
      handles = ["both", "onlyA", "onlyB"];
      break;
    default:
      handles = ["main"];
  }
  if (opts?.errorOutput && !handles.includes("error")) {
    handles = [...handles, "error"];
  }
  return handles;
}

/** Target handle ids for multi-input nodes (edge.targetHandle). */
export function targetHandlesForType(type: string): string[] {
  if (type === "merge" || type === "compareDatasets") return ["0", "1"];
  return ["main"];
}

export function targetHandleLabel(type: string, handle: string): string {
  if (type === "merge" || type === "compareDatasets") {
    if (handle === "0") return "Input 1";
    if (handle === "1") return "Input 2";
  }
  return handle;
}

/** Friendly label for a source (output) handle. */
export function sourceHandleLabel(
  type: string,
  handle: string,
  opts?: { httpMethod?: string },
): string {
  if (handle === "error") return "Error";
  if (
    handle === "main" &&
    (type === "webhook" || type === "apiEndpoint") &&
    opts?.httpMethod
  ) {
    return opts.httpMethod.toUpperCase();
  }
  if (handle === "main") return "Output";
  switch (type) {
    case "if":
      if (handle === "true") return "True";
      if (handle === "false") return "False";
      break;
    case "filter":
    case "removeDuplicates":
      if (handle === "kept") return "Kept";
      if (handle === "discarded") return "Discarded";
      break;
    case "splitInBatches":
      if (handle === "loop") return "Loop";
      if (handle === "done") return "Done";
      break;
    case "compareDatasets":
      if (handle === "both") return "In both";
      if (handle === "onlyA") return "Only A";
      if (handle === "onlyB") return "Only B";
      break;
    default:
      break;
  }
  return handle;
}

/** Idle edge label for webhook / API endpoint sources (n8n shows method on the connection). */
export function connectionLabelForNode(
  type: string,
  parameters?: Record<string, unknown>,
): string | undefined {
  if (type === "webhook") {
    return String(parameters?.method ?? "POST").toUpperCase();
  }
  if (type === "apiEndpoint") {
    return String(parameters?.method ?? "GET").toUpperCase();
  }
  return undefined;
}

/** Tooltip copy for the “+” add-next-step button on an output handle. */
export function addFromHandleTooltip(type: string, handle: string): string {
  const label = sourceHandleLabel(type, handle);
  if (handle === "main" || label === "Output") {
    return "Add next step";
  }
  return `Add next step from “${label}” output`;
}
