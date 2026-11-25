import { getDb } from "../db.js";

export type SsoProtocol = "saml" | "oidc";

export type SamlSettings = {
  metadataUrl: string;
  metadataXml: string;
  metadataSource: "url" | "xml";
  entityId: string;
};

export type OidcSettings = {
  discoveryEndpoint: string;
  clientId: string;
  clientSecret: string;
  prompt: string;
};

export type SsoSettings = {
  loginEnabled: boolean;
  protocol: SsoProtocol;
  saml: SamlSettings;
  oidc: OidcSettings;
};

export type LdapSettings = {
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

export type AuthenticationSettings = {
  sso: SsoSettings;
  ldap: LdapSettings;
  mfaEnforced: boolean;
};

const DOC_ID = "authentication";

export function defaultAuthenticationSettings(): AuthenticationSettings {
  return {
    sso: {
      loginEnabled: false,
      protocol: "saml",
      saml: {
        metadataUrl: "",
        metadataXml: "",
        metadataSource: "url",
        entityId: "",
      },
      oidc: {
        discoveryEndpoint: "",
        clientId: "",
        clientSecret: "",
        prompt: "select_account",
      },
    },
    ldap: {
      loginEnabled: false,
      loginLabel: "LDAP Username or Email Address",
      connectionUrl: "",
      connectionPort: 389,
      connectionSecurity: "none",
      allowUnauthorizedCerts: false,
      baseDn: "",
      bindingAdminDn: "",
      bindingAdminPassword: "",
      userFilter: "",
      loginIdAttribute: "uid",
      emailAttribute: "mail",
      firstNameAttribute: "givenName",
      lastNameAttribute: "sn",
      ldapIdAttribute: "uid",
      synchronizationEnabled: false,
    },
    mfaEnforced: false,
  };
}

function mergeAuthSettings(
  stored: Partial<AuthenticationSettings> | null | undefined,
): AuthenticationSettings {
  const defaults = defaultAuthenticationSettings();
  if (!stored) return defaults;
  return {
    mfaEnforced: Boolean(stored.mfaEnforced),
    sso: {
      ...defaults.sso,
      ...stored.sso,
      saml: { ...defaults.sso.saml, ...stored.sso?.saml },
      oidc: { ...defaults.sso.oidc, ...stored.sso?.oidc },
    },
    ldap: {
      ...defaults.ldap,
      ...stored.ldap,
    },
  };
}

type AuthDoc = { _id: string } & Partial<AuthenticationSettings>;

export async function getAuthenticationSettings(): Promise<AuthenticationSettings> {
  const doc = await getDb()
    .collection<AuthDoc>("instance_settings")
    .findOne({ _id: DOC_ID });
  return mergeAuthSettings(doc ?? undefined);
}

export async function saveAuthenticationSettings(
  next: AuthenticationSettings,
): Promise<AuthenticationSettings> {
  const normalized = mergeAuthSettings(next);
  await getDb()
    .collection<AuthDoc>("instance_settings")
    .updateOne(
      { _id: DOC_ID },
      {
        $set: {
          sso: normalized.sso,
          ldap: normalized.ldap,
          mfaEnforced: normalized.mfaEnforced,
          updatedAt: new Date(),
        },
      },
      { upsert: true },
    );
  return normalized;
}

/** Redact secrets for API responses. */
export function publicAuthenticationSettings(
  settings: AuthenticationSettings,
): AuthenticationSettings {
  return {
    ...settings,
    sso: {
      ...settings.sso,
      oidc: {
        ...settings.sso.oidc,
        clientSecret: settings.sso.oidc.clientSecret ? "********" : "",
      },
    },
    ldap: {
      ...settings.ldap,
      bindingAdminPassword: settings.ldap.bindingAdminPassword
        ? "********"
        : "",
    },
  };
}
