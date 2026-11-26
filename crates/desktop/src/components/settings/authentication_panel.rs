use dioxus::prelude::*;

use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Card, CardContent,
};

#[derive(Clone, Default)]
struct AuthSettingsSnapshot {
    sso_enabled: bool,
    ldap_enabled: bool,
    mfa_enforced: bool,
}

fn parse_auth_settings(v: &serde_json::Value) -> AuthSettingsSnapshot {
    let root = v.get("settings").unwrap_or(v);
    AuthSettingsSnapshot {
        sso_enabled: root
            .pointer("/sso/loginEnabled")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        ldap_enabled: root
            .pointer("/ldap/loginEnabled")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        mfa_enforced: root
            .get("mfaEnforced")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
    }
}

#[component]
fn AuthRow(
    title: &'static str,
    description: &'static str,
    action: String,
    divider: bool,
) -> Element {
    let border = if divider {
        "border-b border-border"
    } else {
        ""
    };
    rsx! {
        div { class: "flex w-full items-start justify-between gap-4 px-4 py-3.5 text-left {border}",
            div { class: "min-w-0 flex-1",
                div { class: "truncate text-sm font-medium text-foreground", "{title}" }
                p { class: "mt-0.5 line-clamp-2 text-xs text-muted-foreground", "{description}" }
            }
            span { class: "mt-0.5 inline-flex shrink-0 items-center text-sm text-foreground",
                "{action}"
            }
        }
    }
}

#[component]
pub fn AuthenticationPanel() -> Element {
    let mut snap = use_signal(AuthSettingsSnapshot::default);
    let mut loading = use_signal(|| true);
    let mut api_available = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);

    use_effect(move || {
        spawn(async move {
            loading.set(true);
            match crate::api::get_json("/api/settings/authentication").await {
                Ok(v) => {
                    snap.set(parse_auth_settings(&v));
                    api_available.set(true);
                    err.set(None);
                }
                Err(e) => {
                    api_available.set(false);
                    err.set(Some(e));
                }
            }
            loading.set(false);
        });
    });

    let settings = snap();
    let sso_action = if loading() {
        "…".to_string()
    } else if settings.sso_enabled {
        "Active".into()
    } else {
        "Configure".into()
    };
    let ldap_action = if loading() {
        "…".to_string()
    } else if settings.ldap_enabled {
        "Active".into()
    } else {
        "Configure".into()
    };
    let mfa_action = if loading() {
        "…".to_string()
    } else if settings.mfa_enforced {
        "Enforced".into()
    } else {
        "Optional".into()
    };

    rsx! {
        div { class: "mx-auto flex h-full w-full max-w-3xl flex-col gap-8 overflow-auto px-8 py-5",
            header {
                h1 { class: "text-[1.75rem] font-normal leading-[1.25]", "Authentication" }
                p { class: "mt-1 text-sm text-muted-foreground",
                    "Choose how people sign in: single sign-on (SSO) through your identity provider, LDAP, or built-in credentials."
                }
            }

            if !api_available() {
                Card { class: Some("border-dashed bg-muted/20".into()),
                    CardContent { class: Some("space-y-2 p-4 text-sm text-muted-foreground".into()),
                        p { class: "font-medium text-foreground", "Embedded BotFlow server" }
                        p {
                            "This desktop build uses local JWT login ("
                            code { class: "rounded bg-muted px-1 text-xs", "/api/v1/auth/login" }
                            "). Full authentication settings are available when connected to the Node/DataFlow API server."
                        }
                        if let Some(message) = err() {
                            Alert {
                                variant: AlertVariant::Destructive,
                                AlertDescription {
                                    class: Some("text-destructive text-xs".into()),
                                    "{message}"
                                }
                            }
                        }
                        ul { class: "list-inside list-disc space-y-1 text-xs",
                            li { "SSO (SAML / OpenID Connect) — planned" }
                            li { "LDAP directory sign-in & sync — planned" }
                            li { "Session policies & MFA enforcement — planned" }
                            li { "Provider-specific configuration UI — planned" }
                        }
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-base font-semibold", "Sign-in methods" }
                Card { class: Some("overflow-hidden p-0 gap-0".into()),
                    AuthRow {
                        title: "Single sign-on (SSO)",
                        description: "Let people sign in through your identity provider with SAML or OpenID Connect.",
                        action: sso_action,
                        divider: true,
                    }
                    AuthRow {
                        title: "LDAP",
                        description: "Let people sign in with their directory account and keep users in sync.",
                        action: ldap_action,
                        divider: false,
                    }
                    AuthRow {
                        title: "Email & password (local)",
                        description: "Built-in account sign-in for this instance.",
                        action: if api_available() {
                            String::from("See Users")
                        } else {
                            String::from("Available")
                        },
                        divider: false,
                    }
                }
                if api_available() {
                    p { class: "text-xs text-muted-foreground",
                        "Detailed SSO and LDAP configuration is managed in the web studio. This view shows current status from "
                        code { class: "rounded bg-muted px-1", "/api/settings/authentication" }
                        "."
                    }
                }
            }

            section { class: "flex flex-col gap-4",
                h2 { class: "text-base font-semibold", "Requirements" }
                Card { class: Some("overflow-hidden p-0 gap-0".into()),
                    AuthRow {
                        title: "Two-factor authentication",
                        description: "Require two-factor authentication for people who sign in with email and password.",
                        action: mfa_action,
                        divider: false,
                    }
                }
            }
        }
    }
}