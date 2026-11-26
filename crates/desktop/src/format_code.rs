//! Code formatting helpers (JSON + passthrough for other languages).

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CodeLanguage {
    #[default]
    Text,
    Sql,
    Json,
    Js,
}

impl CodeLanguage {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "sql" | "tsql" => Self::Sql,
            "json" => Self::Json,
            "js" | "javascript" | "ts" | "typescript" => Self::Js,
            _ => Self::Text,
        }
    }

    pub fn supports_format(self) -> bool {
        matches!(self, Self::Json | Self::Sql)
    }
}

pub struct FormatResult {
    pub ok: bool,
    pub text: String,
    #[allow(dead_code)]
    pub error: Option<String>,
}

pub fn format_json_text(source: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(source).ok()?;
    serde_json::to_string_pretty(&parsed).ok()
}

pub fn format_code(source: &str, language: CodeLanguage) -> FormatResult {
    match language {
        CodeLanguage::Json => match format_json_text(source) {
            Some(text) => FormatResult {
                ok: true,
                text,
                error: None,
            },
            None => FormatResult {
                ok: false,
                text: source.to_string(),
                error: Some("Invalid JSON".into()),
            },
        },
        CodeLanguage::Sql | CodeLanguage::Js | CodeLanguage::Text => FormatResult {
            ok: true,
            text: source.to_string(),
            error: None,
        },
    }
}
