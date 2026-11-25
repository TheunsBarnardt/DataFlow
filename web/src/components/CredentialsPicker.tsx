import { useState } from "react";
import { useCredentials } from "@/hooks/useCredentials";
import { CredentialDialog } from "@/components/CredentialDialog";
import { FormSelect } from "@/components/ui/form-select";
import { getCredentialType } from "@/lib/credentialTypes";
import { cn } from "@/lib/utils";

/** True when a parameter field should use the credentials picker. */
export function isCredentialField(key: string): boolean {
  const k = key.toLowerCase();
  return k === "credentialid" || k.includes("credential");
}

const CREATE_VALUE = "__create__";

export function CredentialsPicker({
  id,
  value,
  onChange,
  className,
  credentialType,
}: {
  id?: string;
  value: string;
  onChange: (next: string) => void;
  className?: string;
  /** When set, only list credentials of this type (or all if empty). */
  credentialType?: string;
}) {
  const { credentials, loading, reload } = useCredentials();
  const [dialogOpen, setDialogOpen] = useState(false);

  const filtered = credentialType
    ? credentials.filter(
        (c) =>
          c.type === credentialType ||
          c.type.toLowerCase().includes(credentialType.toLowerCase()) ||
          credentialType.toLowerCase().includes(c.type.toLowerCase()),
      )
    : credentials;
  const options = filtered.length ? filtered : credentials;

  const typeLabel =
    (credentialType && getCredentialType(credentialType)?.displayName) ||
    credentialType ||
    "credential";

  return (
    <>
      <FormSelect
        id={id}
        className={cn(className)}
        value={value}
        disabled={loading}
        placeholder={loading ? "Loading credentials…" : "Select credential…"}
        options={[
          { value: "", label: loading ? "Loading credentials…" : "Select credential…" },
          ...options.map((c) => ({
            value: c.id,
            label: `${c.name} (${c.type})`,
          })),
          {
            value: CREATE_VALUE,
            label: `+ Create new ${typeLabel}…`,
            separatorBefore: true,
            accent: true,
          },
        ]}
        onChange={(next) => {
          if (next === CREATE_VALUE) {
            setDialogOpen(true);
            return;
          }
          onChange(next);
        }}
      />

      <CredentialDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        preferredType={credentialType}
        onCreated={(cred) => {
          void reload().then(() => onChange(cred.id));
        }}
      />
    </>
  );
}
