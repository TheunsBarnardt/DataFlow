/** Postman-style HTTP method badge colors */
export const HTTP_METHOD_BADGE: Record<string, string> = {
  GET: "border-transparent bg-[#61affe] text-white",
  POST: "border-transparent bg-[#49cc90] text-white",
  PUT: "border-transparent bg-[#fca130] text-white",
  PATCH: "border-transparent bg-[#f39c12] text-white",
  DELETE: "border-transparent bg-[#f93e3e] text-white",
  HEAD: "border-transparent bg-[#c07a4a] text-white",
  OPTIONS: "border-transparent bg-[#0d5aa7] text-white",
};

const METHOD_PREFIX =
  /^(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)(\s+|\/)(.*)$/i;

/** Split a workflow/endpoint name like "DELETE Admin/Foo" into method + rest. */
export function splitHttpMethodName(name: string): {
  method: string | null;
  rest: string;
} {
  const m = name.trim().match(METHOD_PREFIX);
  if (!m) return { method: null, rest: name };
  return { method: m[1].toUpperCase(), rest: m[3] };
}

export function httpMethodBadgeClass(method: string): string {
  return (
    HTTP_METHOD_BADGE[method.toUpperCase()] ??
    "border-transparent bg-muted text-muted-foreground"
  );
}
