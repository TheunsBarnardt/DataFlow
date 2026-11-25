import { config } from "../config.js";
import { getGraphToken, graphFetch } from "../engine/microsoft/graph.js";
import type { NodeContext } from "../engine/types.js";

/**
 * Create a Microsoft Graph subscription pointing at our webhook URL.
 * Requires a credential with client credentials or an access token in env.
 */
export async function ensureGraphSubscription(opts: {
  kind: "outlook" | "teams";
  notificationUrl: string;
  credentialId?: string;
  getCredential: NodeContext["getCredential"];
  getCredentialAsync: NodeContext["getCredentialAsync"];
  getCredentialData: NodeContext["getCredentialData"];
}): Promise<{ id?: string; error?: string }> {
  try {
    const token = await getGraphToken({
      credentialId: opts.credentialId,
      accessToken: process.env.MS_GRAPH_TOKEN,
      getCredential: opts.getCredential,
      getCredentialAsync: opts.getCredentialAsync,
      getCredentialData: opts.getCredentialData,
    });
    if (!token) {
      return { error: "No Graph token for subscription" };
    }
    const resource =
      opts.kind === "outlook"
        ? "/me/mailFolders('Inbox')/messages"
        : "/teams";
    const expiration = new Date(Date.now() + 2 * 24 * 60 * 60_000).toISOString();
    const result = await graphFetch("/subscriptions", token, "POST", {
      changeType: "created,updated",
      notificationUrl: opts.notificationUrl,
      resource,
      expirationDateTime: expiration,
      clientState: config.whatsappVerifyToken || "dataflow",
    });
    const body = result.body as { id?: string; error?: { message?: string } };
    if (result.statusCode >= 400) {
      return {
        error: body?.error?.message || `subscription failed (${result.statusCode})`,
      };
    }
    return { id: body.id };
  } catch (err) {
    return { error: err instanceof Error ? err.message : String(err) };
  }
}
