import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { FormSelect } from "@/components/ui/form-select";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import {
  buildCredentialData,
  CREDENTIAL_TYPES,
  defaultFieldValues,
  fieldIsVisible,
  findCredentialTypes,
  getCredentialType,
  type CredentialTypeDef,
} from "@/lib/credentialTypes";
import { invalidateCredentialsCache } from "@/hooks/useCredentials";
import { cn } from "@/lib/utils";

type Step = "select" | "form";

export type CredentialDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Prefer / lock this type when opening from a node picker */
  preferredType?: string;
  /** Called with the new credential id after save */
  onCreated?: (credential: { id: string; name: string; type: string }) => void;
};

export function CredentialDialog({
  open,
  onOpenChange,
  preferredType,
  onCreated,
}: CredentialDialogProps) {
  const preferred = preferredType
    ? getCredentialType(preferredType) ??
      CREDENTIAL_TYPES.find(
        (t) =>
          t.type.toLowerCase().includes(preferredType.toLowerCase()) ||
          preferredType.toLowerCase().includes(t.type.toLowerCase()),
      )
    : undefined;

  const [step, setStep] = useState<Step>(preferred ? "form" : "select");
  const [typeQuery, setTypeQuery] = useState("");
  const [selectedType, setSelectedType] = useState<CredentialTypeDef | null>(
    preferred ?? null,
  );
  const [name, setName] = useState("");
  const [values, setValues] = useState<
    Record<string, string | number | boolean>
  >({});
  const [sidebar, setSidebar] = useState<"connection" | "details">(
    "connection",
  );
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    const pref = preferredType
      ? getCredentialType(preferredType) ??
        CREDENTIAL_TYPES.find(
          (t) =>
            t.type.toLowerCase().includes(preferredType.toLowerCase()) ||
            preferredType.toLowerCase().includes(t.type.toLowerCase()),
        )
      : undefined;
    setStep(pref ? "form" : "select");
    setSelectedType(pref ?? null);
    setTypeQuery("");
    setName(pref ? `${pref.displayName} account` : "");
    setValues(pref ? defaultFieldValues(pref) : {});
    setSidebar("connection");
  }, [open, preferredType]);

  const filteredTypes = useMemo(
    () => findCredentialTypes(typeQuery),
    [typeQuery],
  );

  function pickType(def: CredentialTypeDef) {
    setSelectedType(def);
    setName(`${def.displayName} account`);
    setValues(defaultFieldValues(def));
    setStep("form");
    setSidebar("connection");
  }

  async function save() {
    if (!selectedType) return;
    const trimmed = name.trim();
    if (!trimmed) {
      toast.error("Name is required");
      return;
    }
    for (const f of selectedType.fields) {
      if (!f.required || f.kind === "boolean") continue;
      if (!fieldIsVisible(f, values)) continue;
      const v = values[f.key];
      if (v === undefined || v === "") {
        toast.error(`${f.label} is required`);
        return;
      }
    }
    let data: Record<string, unknown>;
    try {
      data = buildCredentialData(selectedType, values);
    } catch {
      toast.error("Secret data must be valid JSON");
      return;
    }
    setBusy(true);
    try {
      const res = await api<{
        credential: { id: string; name: string; type: string };
      }>("/api/credentials", {
        method: "POST",
        body: JSON.stringify({
          name: trimmed,
          type: selectedType.type,
          data,
        }),
      });
      invalidateCredentialsCache();
      toast.success("Credential created");
      onCreated?.(res.credential);
      onOpenChange(false);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        showCloseButton
        className={cn(
          "flex max-h-[min(90vh,720px)] flex-col gap-0 overflow-hidden p-0",
          step === "select" ? "sm:max-w-md" : "sm:max-w-3xl",
        )}
      >
        {step === "select" ? (
          <>
            <DialogHeader className="border-b px-5 py-4 text-left">
              <DialogTitle>Add new credential</DialogTitle>
              <DialogDescription>
                Select an app or service to connect to
              </DialogDescription>
            </DialogHeader>
            <div className="flex min-h-0 flex-1 flex-col gap-3 p-5">
              <Input
                autoFocus
                placeholder="Search apps and services…"
                value={typeQuery}
                onChange={(e) => setTypeQuery(e.target.value)}
              />
              <ScrollArea className="h-[min(40vh,320px)] rounded-md border">
                <div className="p-0">
                  {filteredTypes.map((t) => (
                    <Button
                      key={t.type}
                      type="button"
                      variant="ghost"
                      className={cn(
                        "h-auto w-full flex-col items-start gap-0.5 rounded-none border-b px-3 py-2.5 text-left last:border-b-0",
                        selectedType?.type === t.type && "bg-accent",
                      )}
                      onClick={() => pickType(t)}
                    >
                      <span className="text-sm font-medium">
                        {t.displayName}
                      </span>
                      <span className="text-xs font-normal text-muted-foreground">
                        {t.category}
                        {t.description ? ` · ${t.description}` : ""}
                      </span>
                    </Button>
                  ))}
                  {!filteredTypes.length ? (
                    <div className="px-3 py-8 text-center text-sm text-muted-foreground">
                      No matching credential types
                    </div>
                  ) : null}
                </div>
              </ScrollArea>
            </div>
            <DialogFooter className="border-t px-5 py-3">
              <Button variant="ghost" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button
                disabled={!selectedType}
                onClick={() => selectedType && pickType(selectedType)}
              >
                Continue
              </Button>
            </DialogFooter>
          </>
        ) : selectedType ? (
          <>
            <DialogHeader className="flex flex-row items-start justify-between gap-3 space-y-0 border-b px-5 py-4 text-left">
              <div className="min-w-0 flex-1">
                <DialogTitle className="truncate">
                  {name.trim() || `${selectedType.displayName} account`}
                </DialogTitle>
                <DialogDescription>
                  {selectedType.displayName}
                </DialogDescription>
              </div>
              <div className="flex shrink-0 gap-2 pr-6">
                {!preferredType ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setStep("select")}
                  >
                    Change type
                  </Button>
                ) : null}
                <Button size="sm" disabled={busy} onClick={() => void save()}>
                  Save
                </Button>
              </div>
            </DialogHeader>

            <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[170px_1fr]">
              <aside className="flex flex-row gap-1 border-b p-3 md:flex-col md:border-b-0 md:border-r">
                {(
                  [
                    ["connection", "Connection"],
                    ["details", "Details"],
                  ] as const
                ).map(([id, label]) => (
                  <Button
                    key={id}
                    type="button"
                    variant={sidebar === id ? "secondary" : "ghost"}
                    size="sm"
                    className="justify-start"
                    onClick={() => setSidebar(id)}
                  >
                    {label}
                  </Button>
                ))}
              </aside>

              <ScrollArea className="max-h-[min(55vh,480px)]">
                <div className="p-5">
                  <FieldGroup className="max-w-lg gap-4">
                    <Field>
                      <FieldLabel htmlFor="cred-name">
                        Credential name
                      </FieldLabel>
                      <Input
                        id="cred-name"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder={`${selectedType.displayName} account`}
                      />
                    </Field>

                    {sidebar === "details" ? (
                      <FieldDescription>
                        Type:{" "}
                        <code className="text-foreground">
                          {selectedType.type}
                        </code>
                      </FieldDescription>
                    ) : (
                      selectedType.fields.map((f) => {
                        if (!fieldIsVisible(f, values)) return null;
                        const fieldId = `cred-${f.key}`;
                        if (f.kind === "boolean") {
                          return (
                            <Field
                              key={f.key}
                              orientation="horizontal"
                              className="items-center"
                            >
                              <FieldLabel htmlFor={fieldId}>
                                {f.label}
                              </FieldLabel>
                              <Switch
                                id={fieldId}
                                checked={Boolean(values[f.key])}
                                onCheckedChange={(checked) =>
                                  setValues((v) => ({
                                    ...v,
                                    [f.key]: checked,
                                  }))
                                }
                              />
                              {f.description ? (
                                <FieldDescription className="basis-full">
                                  {f.description}
                                </FieldDescription>
                              ) : null}
                            </Field>
                          );
                        }
                        return (
                          <Field key={f.key}>
                            <FieldLabel htmlFor={fieldId}>
                              {f.label}
                              {f.required ? " *" : ""}
                            </FieldLabel>
                            {f.kind === "textarea" ? (
                              <Textarea
                                id={fieldId}
                                className="min-h-[88px] font-mono text-xs"
                                placeholder={f.placeholder}
                                value={String(values[f.key] ?? "")}
                                onChange={(e) =>
                                  setValues((v) => ({
                                    ...v,
                                    [f.key]: e.target.value,
                                  }))
                                }
                                spellCheck={false}
                              />
                            ) : f.kind === "select" ? (
                              <FormSelect
                                id={fieldId}
                                value={String(values[f.key] ?? "")}
                                options={f.options ?? []}
                                onChange={(next) =>
                                  setValues((v) => ({
                                    ...v,
                                    [f.key]: next,
                                  }))
                                }
                              />
                            ) : (
                              <Input
                                id={fieldId}
                                type={
                                  f.kind === "password"
                                    ? "password"
                                    : f.kind === "number"
                                      ? "number"
                                      : "text"
                                }
                                placeholder={f.placeholder}
                                value={String(values[f.key] ?? "")}
                                onChange={(e) =>
                                  setValues((v) => ({
                                    ...v,
                                    [f.key]:
                                      f.kind === "number"
                                        ? e.target.value === ""
                                          ? ""
                                          : Number(e.target.value)
                                        : e.target.value,
                                  }))
                                }
                              />
                            )}
                            {f.description ? (
                              <FieldDescription>
                                {f.description}
                              </FieldDescription>
                            ) : null}
                          </Field>
                        );
                      })
                    )}
                  </FieldGroup>
                </div>
              </ScrollArea>
            </div>

            <DialogFooter className="border-t px-5 py-3">
              <Button variant="ghost" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button disabled={busy} onClick={() => void save()}>
                Save
              </Button>
            </DialogFooter>
          </>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
