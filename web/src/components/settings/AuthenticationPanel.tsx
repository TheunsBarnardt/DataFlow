import { useCallback, useEffect, useState, type ReactNode } from "react";
import { ArrowLeft, ChevronRight } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Field as UiField,
  FieldDescription,
  FieldLabel,
} from "@/components/ui/field";
import { FormSelect } from "@/components/ui/form-select";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import type { SettingsPageId } from "@/components/settings/settingsNav";

const AUTH_DOCS_URL =
  "https://docs.n8n.io/administer/manage-users-and-access/verify-user-identity";
const SSO_DOCS_URL = "https://docs.n8n.io/user-management/saml/";
const LDAP_DOCS_URL = "https://docs.n8n.io/user-management/ldap/";

type SsoProtocol = "saml" | "oidc";

type AuthenticationSettings = {
  sso: {
    loginEnabled: boolean;
    protocol: SsoProtocol;
    saml: {
      metadataUrl: string;
      metadataXml: string;
      metadataSource: "url" | "xml";
      entityId: string;
    };
    oidc: {
      discoveryEndpoint: string;
      clientId: string;
      clientSecret: string;
      prompt: string;
    };
  };
  ldap: {
    loginEnabled: boolean;
    loginLabel: string;
    connectionUrl: string;
    connectionPort: number;
    connectionSecurity: "none" | "tls" | "startTls";
    allowUnauthorizedCerts: boolean;
    baseDn: string;
    bindingAdminDn: string;
    bindingAdminPassword: string;
    userFilter: string;
    loginIdAttribute: string;
    emailAttribute: string;
    firstNameAttribute: string;
    lastNameAttribute: string;
    ldapIdAttribute: string;
    synchronizationEnabled: boolean;
  };
  mfaEnforced: boolean;
};

type AuthView = "hub" | "sso" | "ldap";

function DocsLink({ href }: { href: string }) {
  return (
    <>
      {" "}
      Learn more in the{" "}
      <a
        href={href}
        target="_blank"
        rel="noreferrer"
        className="underline underline-offset-2 hover:text-foreground"
      >
        documentation
      </a>
    </>
  );
}

function SettingsRow({
  title,
  description,
  action,
  onClick,
  divider,
}: {
  title: string;
  description: string;
  action: string;
  onClick: () => void;
  divider?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex w-full items-start justify-between gap-4 px-4 py-3.5 text-left transition-colors hover:bg-muted/50 ${
        divider ? "border-b border-border" : ""
      }`}
    >
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium text-foreground">
          {title}
        </div>
        <p className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">
          {description}
        </p>
      </div>
      <span className="mt-0.5 inline-flex shrink-0 items-center gap-0.5 text-sm text-foreground">
        {action}
        <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
      </span>
    </button>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <UiField>
      <FieldLabel>{label}</FieldLabel>
      {children}
      {hint ? <FieldDescription>{hint}</FieldDescription> : null}
    </UiField>
  );
}

function selectClassName() {
  return "flex h-9 w-full rounded-md border bg-background px-3 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring";
}

function HubView({
  settings,
  onOpenSso,
  onOpenLdap,
  onOpenPolicies,
}: {
  settings: AuthenticationSettings | null;
  onOpenSso: () => void;
  onOpenLdap: () => void;
  onOpenPolicies: () => void;
}) {
  const ssoActive = Boolean(settings?.sso.loginEnabled);
  const ldapActive = Boolean(settings?.ldap.loginEnabled);
  const mfaLabel = settings?.mfaEnforced ? "Enforced" : "Optional";

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col gap-8 overflow-auto p-6">
      <header>
        <h1 className="text-[1.75rem] font-normal leading-[1.25]">Authentication</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Choose how people sign in to this instance: single sign-on (SSO)
          through your identity provider, or LDAP.
          <DocsLink href={AUTH_DOCS_URL} />
        </p>
      </header>

      <section className="flex flex-col gap-4">
        <h2 className="text-base font-semibold">Sign-in methods</h2>
        <div className="overflow-hidden rounded-md border bg-card">
          <SettingsRow
            title="Single sign-on (SSO)"
            description="Let people sign in through your identity provider with SAML or OpenID Connect."
            action={ssoActive ? "Active" : "Configure"}
            onClick={onOpenSso}
            divider
          />
          <SettingsRow
            title="LDAP"
            description="Let people sign in with their directory account and keep users in sync."
            action={ldapActive ? "Active" : "Configure"}
            onClick={onOpenLdap}
          />
        </div>
      </section>

      <section className="flex flex-col gap-4">
        <h2 className="text-base font-semibold">Requirements</h2>
        <div className="overflow-hidden rounded-md border bg-card">
          <SettingsRow
            title="Two-factor authentication"
            description="Require two-factor authentication for people who sign in with email and password. Managed in Policies."
            action={mfaLabel}
            onClick={onOpenPolicies}
          />
        </div>
      </section>
    </div>
  );
}

function BackLink({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mb-4 inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
    >
      <ArrowLeft className="h-3.5 w-3.5" />
      Back to Authentication
    </button>
  );
}

function SsoView({
  settings,
  onBack,
  onSaved,
}: {
  settings: AuthenticationSettings;
  onBack: () => void;
  onSaved: (next: AuthenticationSettings) => void;
}) {
  const [draft, setDraft] = useState(settings.sso);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setDraft(settings.sso);
  }, [settings]);

  async function save() {
    setBusy(true);
    try {
      const res = await api<{ settings: AuthenticationSettings }>(
        "/api/settings/authentication",
        {
          method: "PUT",
          body: JSON.stringify({ sso: draft }),
        },
      );
      onSaved(res.settings);
      toast.success("SSO settings saved");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col overflow-auto p-6">
      <BackLink onClick={onBack} />
      <header className="mb-6">
        <h1 className="text-[1.75rem] font-normal leading-[1.25]">
          Single Sign-On
        </h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Configure SSO to let your team sign in using your identity provider.
          Supports SAML 2.0 and OpenID Connect protocols.
          <DocsLink href={SSO_DOCS_URL} />
        </p>
      </header>

      <div className="flex flex-col gap-5 rounded-md border bg-card p-4">
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium">Enable SSO login</div>
            <p className="text-xs text-muted-foreground">
              When enabled, people can sign in through your identity provider.
              Enabling SSO turns off LDAP login.
            </p>
          </div>
          <Switch
            checked={draft.loginEnabled}
            onCheckedChange={(checked) =>
              setDraft((d) => ({ ...d, loginEnabled: checked }))
            }
          />
        </div>

        <Field
          label="Authentication protocol"
          hint="Choose your SSO protocol"
        >
          <FormSelect
            className={selectClassName()}
            value={draft.protocol}
            options={[
              { value: "saml", label: "SAML" },
              { value: "oidc", label: "OIDC" },
            ]}
            onChange={(next) =>
              setDraft((d) => ({
                ...d,
                protocol: next as SsoProtocol,
              }))
            }
          />
        </Field>

        {draft.protocol === "saml" ? (
          <>
            <Field label="Identity provider settings">
              <FormSelect
                className={selectClassName()}
                value={draft.saml.metadataSource}
                options={[
                  { value: "url", label: "Metadata URL" },
                  { value: "xml", label: "Metadata XML" },
                ]}
                onChange={(next) =>
                  setDraft((d) => ({
                    ...d,
                    saml: {
                      ...d.saml,
                      metadataSource: next as "url" | "xml",
                    },
                  }))
                }
              />
            </Field>
            {draft.saml.metadataSource === "url" ? (
              <Field
                label="Metadata URL"
                hint="URL where your IdP publishes SAML metadata"
              >
                <Input
                  value={draft.saml.metadataUrl}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      saml: { ...d.saml, metadataUrl: e.target.value },
                    }))
                  }
                  placeholder="https://idp.example.com/metadata"
                />
              </Field>
            ) : (
              <Field label="Metadata XML">
                <Textarea
                  className="min-h-28 font-mono text-xs"
                  value={draft.saml.metadataXml}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      saml: { ...d.saml, metadataXml: e.target.value },
                    }))
                  }
                  placeholder="<EntityDescriptor …>"
                />
              </Field>
            )}
            <Field
              label="Entity ID"
              hint="Optional override for the SAML entity ID"
            >
              <Input
                value={draft.saml.entityId}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    saml: { ...d.saml, entityId: e.target.value },
                  }))
                }
              />
            </Field>
          </>
        ) : (
          <>
            <Field
              label="Discovery endpoint"
              hint="OpenID Connect discovery document URL"
            >
              <Input
                value={draft.oidc.discoveryEndpoint}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    oidc: { ...d.oidc, discoveryEndpoint: e.target.value },
                  }))
                }
                placeholder="https://idp.example.com/.well-known/openid-configuration"
              />
            </Field>
            <Field label="Client ID">
              <Input
                value={draft.oidc.clientId}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    oidc: { ...d.oidc, clientId: e.target.value },
                  }))
                }
              />
            </Field>
            <Field
              label="Client secret"
              hint="Leave as ******** to keep the stored secret"
            >
              <Input
                type="password"
                value={draft.oidc.clientSecret}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    oidc: { ...d.oidc, clientSecret: e.target.value },
                  }))
                }
              />
            </Field>
            <Field label="Prompt">
              <FormSelect
                className={selectClassName()}
                value={draft.oidc.prompt}
                options={[
                  { value: "login", label: "login" },
                  { value: "none", label: "none" },
                  { value: "consent", label: "consent" },
                  { value: "select_account", label: "select_account" },
                  { value: "create", label: "create" },
                ]}
                onChange={(next) =>
                  setDraft((d) => ({
                    ...d,
                    oidc: { ...d.oidc, prompt: next },
                  }))
                }
              />
            </Field>
          </>
        )}
      </div>

      <div className="mt-6 flex gap-2">
        <Button onClick={() => void save()} disabled={busy}>
          {busy ? "Saving…" : "Save"}
        </Button>
      </div>

      <p className="mt-4 text-xs text-muted-foreground">
        Settings are stored for this instance. Sign-in via SSO is applied when
        the identity provider integration is wired into Better Auth.
      </p>
    </div>
  );
}

function LdapView({
  settings,
  onBack,
  onSaved,
}: {
  settings: AuthenticationSettings;
  onBack: () => void;
  onSaved: (next: AuthenticationSettings) => void;
}) {
  const [draft, setDraft] = useState(settings.ldap);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setDraft(settings.ldap);
  }, [settings]);

  async function save() {
    setBusy(true);
    try {
      const res = await api<{ settings: AuthenticationSettings }>(
        "/api/settings/authentication",
        {
          method: "PUT",
          body: JSON.stringify({ ldap: draft }),
        },
      );
      onSaved(res.settings);
      toast.success("LDAP settings saved");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col overflow-auto p-6">
      <BackLink onClick={onBack} />
      <header className="mb-6">
        <h1 className="text-[1.75rem] font-normal leading-[1.25]">LDAP</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Let people sign in with their directory account and keep users in
          sync.
          <DocsLink href={LDAP_DOCS_URL} />
        </p>
      </header>

      <div className="flex flex-col gap-5 rounded-md border bg-card p-4">
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium">Enable LDAP login</div>
            <p className="text-xs text-muted-foreground">
              Enabling LDAP turns off SSO login.
            </p>
          </div>
          <Switch
            checked={draft.loginEnabled}
            onCheckedChange={(checked) =>
              setDraft((d) => ({ ...d, loginEnabled: checked }))
            }
          />
        </div>

        {draft.loginEnabled ? (
          <>
            <Field
              label="Login label"
              hint="Placeholder shown on the login form"
            >
              <Input
                value={draft.loginLabel}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, loginLabel: e.target.value }))
                }
              />
            </Field>
            <Field label="LDAP server address">
              <Input
                value={draft.connectionUrl}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, connectionUrl: e.target.value }))
                }
                placeholder="ldap.example.com"
              />
            </Field>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field label="Port">
                <Input
                  type="number"
                  value={draft.connectionPort}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      connectionPort: Number(e.target.value) || 389,
                    }))
                  }
                />
              </Field>
              <Field label="Connection security">
                <FormSelect
                  className={selectClassName()}
                  value={draft.connectionSecurity}
                  options={[
                    { value: "none", label: "None" },
                    { value: "tls", label: "TLS" },
                    { value: "startTls", label: "STARTTLS" },
                  ]}
                  onChange={(next) =>
                    setDraft((d) => ({
                      ...d,
                      connectionSecurity: next as "none" | "tls" | "startTls",
                    }))
                  }
                />
              </Field>
            </div>
            {draft.connectionSecurity !== "none" ? (
              <div className="flex items-center justify-between gap-4">
                <div className="text-sm font-medium">
                  Ignore SSL/TLS issues
                </div>
                <Switch
                  checked={draft.allowUnauthorizedCerts}
                  onCheckedChange={(checked) =>
                    setDraft((d) => ({
                      ...d,
                      allowUnauthorizedCerts: checked,
                    }))
                  }
                />
              </div>
            ) : null}
            <Field label="Base DN" hint="e.g. dc=example,dc=com">
              <Input
                value={draft.baseDn}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, baseDn: e.target.value }))
                }
              />
            </Field>
            <Field label="Binding admin DN">
              <Input
                value={draft.bindingAdminDn}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, bindingAdminDn: e.target.value }))
                }
              />
            </Field>
            <Field
              label="Binding admin password"
              hint="Leave as ******** to keep the stored password"
            >
              <Input
                type="password"
                value={draft.bindingAdminPassword}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    bindingAdminPassword: e.target.value,
                  }))
                }
              />
            </Field>
            <Field label="User filter">
              <Input
                value={draft.userFilter}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, userFilter: e.target.value }))
                }
                placeholder="(objectClass=person)"
              />
            </Field>

            <div className="border-t pt-4 text-sm font-semibold">
              Attribute mapping
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field label="ID">
                <Input
                  value={draft.ldapIdAttribute}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      ldapIdAttribute: e.target.value,
                    }))
                  }
                />
              </Field>
              <Field label="Login ID">
                <Input
                  value={draft.loginIdAttribute}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      loginIdAttribute: e.target.value,
                    }))
                  }
                />
              </Field>
              <Field label="Email">
                <Input
                  value={draft.emailAttribute}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      emailAttribute: e.target.value,
                    }))
                  }
                />
              </Field>
              <Field label="First name">
                <Input
                  value={draft.firstNameAttribute}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      firstNameAttribute: e.target.value,
                    }))
                  }
                />
              </Field>
              <Field label="Last name">
                <Input
                  value={draft.lastNameAttribute}
                  onChange={(e) =>
                    setDraft((d) => ({
                      ...d,
                      lastNameAttribute: e.target.value,
                    }))
                  }
                />
              </Field>
            </div>

            <div className="flex items-center justify-between gap-4 border-t pt-4">
              <div>
                <div className="text-sm font-medium">
                  Enable periodic synchronization
                </div>
                <p className="text-xs text-muted-foreground">
                  Sync directory users into this instance on a schedule.
                </p>
              </div>
              <Switch
                checked={draft.synchronizationEnabled}
                onCheckedChange={(checked) =>
                  setDraft((d) => ({
                    ...d,
                    synchronizationEnabled: checked,
                  }))
                }
              />
            </div>
          </>
        ) : null}
      </div>

      <div className="mt-6 flex gap-2">
        <Button onClick={() => void save()} disabled={busy}>
          {busy ? "Saving…" : "Save"}
        </Button>
      </div>

      <p className="mt-4 text-xs text-muted-foreground">
        Settings are stored for this instance. Directory sign-in is applied when
        LDAP authentication is wired into Better Auth.
      </p>
    </div>
  );
}

export function AuthenticationPanel({
  onNavigate,
}: {
  onNavigate?: (page: SettingsPageId) => void;
}) {
  const [view, setView] = useState<AuthView>("hub");
  const [settings, setSettings] = useState<AuthenticationSettings | null>(
    null,
  );
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api<{ settings: AuthenticationSettings }>(
        "/api/settings/authentication",
      );
      setSettings(res.settings);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  if (loading && !settings) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Loading…
      </div>
    );
  }

  if (view === "sso" && settings) {
    return (
      <SsoView
        settings={settings}
        onBack={() => setView("hub")}
        onSaved={(next) => setSettings(next)}
      />
    );
  }

  if (view === "ldap" && settings) {
    return (
      <LdapView
        settings={settings}
        onBack={() => setView("hub")}
        onSaved={(next) => setSettings(next)}
      />
    );
  }

  return (
    <HubView
      settings={settings}
      onOpenSso={() => setView("sso")}
      onOpenLdap={() => setView("ldap")}
      onOpenPolicies={() => onNavigate?.("policies")}
    />
  );
}
