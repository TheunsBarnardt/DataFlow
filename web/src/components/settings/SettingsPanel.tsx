import { ProfilePanel } from "@/components/auth/ProfilePanel";
import { UsersPanel } from "@/components/UsersPanel";
import { AuthenticationPanel } from "@/components/settings/AuthenticationPanel";
import { RolesPanel } from "@/components/settings/RolesPanel";
import type { SettingsPageId } from "@/components/settings/settingsNav";

export type { SettingsPageId };

function StubPage({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <div className="mx-auto flex h-full max-w-[1600px] flex-col gap-3 overflow-auto px-8 py-5">
      <div>
        <h1 className="text-[1.75rem] font-normal leading-[1.25]">{title}</h1>
        <p className="mt-3 text-sm leading-[1.35] text-muted-foreground">
          {description}
        </p>
      </div>
      <div className="rounded-lg border border-dashed p-6 text-sm text-muted-foreground">
        Coming soon.
      </div>
    </div>
  );
}

export function SettingsPanel({
  page,
  isAdmin,
  onNavigate,
}: {
  page: SettingsPageId;
  isAdmin: boolean;
  onNavigate?: (page: SettingsPageId) => void;
}) {
  return (
    <div className="h-full min-h-0 overflow-hidden">
      {page === "profile" ? <ProfilePanel /> : null}
      {page === "users" && isAdmin ? <UsersPanel /> : null}
      {page === "roles" ? <RolesPanel /> : null}
      {page === "authentication" ? (
        <AuthenticationPanel onNavigate={onNavigate} />
      ) : null}
      {page === "mcp" ? (
        <StubPage
          title="Instance-level MCP"
          description="Expose DataFlow tools over MCP for agents and IDE clients."
        />
      ) : null}
      {page === "assistant" ? (
        <StubPage
          title="Assistant"
          description="Configure the in-app workflow assistant."
        />
      ) : null}
      {page === "agents" ? (
        <StubPage
          title="Agents"
          description="Preview settings for autonomous agent runs."
        />
      ) : null}
      {page === "context" ? (
        <StubPage
          title="Context"
          description="Preview controls for shared agent context."
        />
      ) : null}
      {page === "chat" ? (
        <StubPage
          title="Chat"
          description="Chat surface settings for assistants and agents."
        />
      ) : null}
      {page === "api" ? (
        <StubPage
          title="DataFlow API"
          description="Instance API keys and public API access."
        />
      ) : null}
      {page === "community-nodes" ? (
        <StubPage
          title="Community nodes"
          description="Install and manage community node packages."
        />
      ) : null}
      {page === "policies" ? (
        <StubPage
          title="Policies"
          description="Security policies for credentials, sharing, and execution."
        />
      ) : null}
    </div>
  );
}
