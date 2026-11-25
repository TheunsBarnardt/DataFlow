import type { FastifyInstance } from "fastify";
import { requireSession, requireAdmin } from "./registerAuth.js";
import {
  getAuthenticationSettings,
  publicAuthenticationSettings,
  saveAuthenticationSettings,
  type AuthenticationSettings,
  type LdapSettings,
  type SsoSettings,
} from "./authenticationSettings.js";

const SECRET_PLACEHOLDER = "********";

function applySecret(
  incoming: string | undefined,
  current: string,
): string {
  if (incoming === undefined) return current;
  if (incoming === SECRET_PLACEHOLDER) return current;
  return incoming;
}

export async function registerAuthenticationRoutes(
  app: FastifyInstance,
): Promise<void> {
  app.get("/api/settings/authentication", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    if (!requireAdmin(req, reply)) return;
    const settings = await getAuthenticationSettings();
    return { settings: publicAuthenticationSettings(settings) };
  });

  app.put<{
    Body: {
      sso?: Partial<SsoSettings> & {
        saml?: Partial<SsoSettings["saml"]>;
        oidc?: Partial<SsoSettings["oidc"]>;
      };
      ldap?: Partial<LdapSettings>;
      mfaEnforced?: boolean;
    };
  }>("/api/settings/authentication", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    if (!requireAdmin(req, reply)) return;

    const current = await getAuthenticationSettings();
    const body = req.body ?? {};
    const next: AuthenticationSettings = {
      mfaEnforced:
        body.mfaEnforced !== undefined
          ? Boolean(body.mfaEnforced)
          : current.mfaEnforced,
      sso: current.sso,
      ldap: current.ldap,
    };

    if (body.sso) {
      next.sso = {
        ...current.sso,
        ...body.sso,
        saml: { ...current.sso.saml, ...body.sso.saml },
        oidc: {
          ...current.sso.oidc,
          ...body.sso.oidc,
          clientSecret: applySecret(
            body.sso.oidc?.clientSecret,
            current.sso.oidc.clientSecret,
          ),
        },
      };
      if (next.sso.loginEnabled) {
        next.ldap = { ...next.ldap, loginEnabled: false };
      }
    }

    if (body.ldap) {
      next.ldap = {
        ...current.ldap,
        ...body.ldap,
        bindingAdminPassword: applySecret(
          body.ldap.bindingAdminPassword,
          current.ldap.bindingAdminPassword,
        ),
      };
      if (next.ldap.loginEnabled) {
        next.sso = { ...next.sso, loginEnabled: false };
      }
    }

    const settings = await saveAuthenticationSettings(next);
    return { settings: publicAuthenticationSettings(settings) };
  });
}
