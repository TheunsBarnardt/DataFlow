//! Code formatting helpers (JSON + SQL).

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

/// Lightweight SQL pretty-printer. Protects string literals, comments, and
/// BotFlow `={{…}}` / `{{…}}` expression islands so they survive reformatting.
pub fn format_sql_text(source: &str) -> String {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let (masked, tokens) = mask_sql_protected(trimmed);
    let pretty = pretty_print_sql(&masked);
    unmask_sql_protected(&pretty, &tokens)
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
        CodeLanguage::Sql => {
            let text = format_sql_text(source);
            FormatResult {
                ok: true,
                text,
                error: None,
            }
        }
        CodeLanguage::Js | CodeLanguage::Text => FormatResult {
            ok: true,
            text: source.to_string(),
            error: None,
        },
    }
}

fn mask_sql_protected(source: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(source.len());
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // BotFlow expressions: ={{ … }} or {{ … }}
        if bytes[i] == b'=' && i + 2 < bytes.len() && &bytes[i..i + 3] == b"={{" {
            if let Some(end) = find_expr_end(bytes, i + 3) {
                push_token(&mut out, &mut tokens, &source[i..end]);
                i = end;
                continue;
            }
        }
        if i + 1 < bytes.len() && &bytes[i..i + 2] == b"{{" {
            if let Some(end) = find_expr_end(bytes, i + 2) {
                push_token(&mut out, &mut tokens, &source[i..end]);
                i = end;
                continue;
            }
        }

        // Line comment
        if i + 1 < bytes.len() && &bytes[i..i + 2] == b"--" {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j] != b'\n' {
                j += 1;
            }
            push_token(&mut out, &mut tokens, &source[i..j]);
            i = j;
            continue;
        }

        // Block comment
        if i + 1 < bytes.len() && &bytes[i..i + 2] == b"/*" {
            let mut j = i + 2;
            while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                j += 1;
            }
            j = (j + 2).min(bytes.len());
            push_token(&mut out, &mut tokens, &source[i..j]);
            i = j;
            continue;
        }

        // Single-quoted string (SQL) with '' escapes
        if bytes[i] == b'\'' {
            let mut j = i + 1;
            while j < bytes.len() {
                if bytes[j] == b'\'' {
                    if j + 1 < bytes.len() && bytes[j + 1] == b'\'' {
                        j += 2;
                        continue;
                    }
                    j += 1;
                    break;
                }
                j += 1;
            }
            push_token(&mut out, &mut tokens, &source[i..j]);
            i = j;
            continue;
        }

        // Bracketed identifier [Name]
        if bytes[i] == b'[' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b']' {
                j += 1;
            }
            if j < bytes.len() {
                j += 1;
            }
            push_token(&mut out, &mut tokens, &source[i..j]);
            i = j;
            continue;
        }

        out.push(bytes[i] as char);
        i += 1;
    }

    (out, tokens)
}

fn find_expr_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut j = from;
    while j + 1 < bytes.len() {
        if bytes[j] == b'}' && bytes[j + 1] == b'}' {
            return Some(j + 2);
        }
        j += 1;
    }
    None
}

fn push_token(out: &mut String, tokens: &mut Vec<String>, slice: &str) {
    let idx = tokens.len();
    tokens.push(slice.to_string());
    out.push_str(&format!("__BFSQL{idx}__"));
}

fn unmask_sql_protected(source: &str, tokens: &[String]) -> String {
    let mut out = source.to_string();
    for (i, tok) in tokens.iter().enumerate() {
        let key = format!("__BFSQL{i}__");
        out = out.replace(&key, tok);
    }
    out
}

fn pretty_print_sql(source: &str) -> String {
    // Collapse whitespace outside protected tokens (already masked).
    let compact = collapse_ws(source);
    let upper = compact.to_ascii_uppercase();

    let mut out = String::with_capacity(compact.len() + 32);
    let mut i = 0;
    let mut indent: usize = 0;
    let mut after_select = false;
    let chars: Vec<char> = compact.chars().collect();

    while i < chars.len() {
        // Multi-word keywords first
        if let Some((kw, len)) = match_keyword_at(&upper, i) {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            // Dedent before clause keywords (except opening SELECT/WITH/…)
            if matches!(
                kw,
                "FROM"
                    | "WHERE"
                    | "GROUP BY"
                    | "ORDER BY"
                    | "HAVING"
                    | "LIMIT"
                    | "OFFSET"
                    | "UNION"
                    | "UNION ALL"
                    | "INTERSECT"
                    | "EXCEPT"
                    | "RETURNING"
                    | "SET"
                    | "VALUES"
                    | "ON"
                    | "JOIN"
                    | "LEFT JOIN"
                    | "RIGHT JOIN"
                    | "INNER JOIN"
                    | "OUTER JOIN"
                    | "FULL JOIN"
                    | "CROSS JOIN"
                    | "LEFT OUTER JOIN"
                    | "RIGHT OUTER JOIN"
                    | "FULL OUTER JOIN"
            ) {
                indent = 0;
                after_select = false;
            }
            if matches!(kw, "SELECT" | "WITH" | "INSERT" | "UPDATE" | "DELETE") {
                indent = 0;
                after_select = kw == "SELECT";
            }
            push_indent(&mut out, indent);
            // Preserve original keyword casing from source slice when possible;
            // prefer uppercase for readability.
            out.push_str(kw);
            i += len;
            // Skip following spaces in compact stream
            while i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            if after_select && kw == "SELECT" {
                out.push('\n');
                indent = 1;
                push_indent(&mut out, indent);
            } else if matches!(
                kw,
                "FROM"
                    | "WHERE"
                    | "GROUP BY"
                    | "ORDER BY"
                    | "HAVING"
                    | "JOIN"
                    | "LEFT JOIN"
                    | "RIGHT JOIN"
                    | "INNER JOIN"
                    | "OUTER JOIN"
                    | "FULL JOIN"
                    | "CROSS JOIN"
                    | "LEFT OUTER JOIN"
                    | "RIGHT OUTER JOIN"
                    | "FULL OUTER JOIN"
                    | "SET"
                    | "VALUES"
                    | "ON"
            ) {
                out.push(' ');
                indent = 1;
            } else {
                out.push(' ');
            }
            continue;
        }

        let ch = chars[i];
        if ch == ',' && after_select {
            out.push(',');
            out.push('\n');
            push_indent(&mut out, indent);
            i += 1;
            while i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }

        if ch == ';' {
            after_select = false;
            indent = 0;
            out.push(';');
            i += 1;
            if i < chars.len() {
                out.push('\n');
            }
            continue;
        }

        out.push(ch);
        i += 1;
    }

    out.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn collapse_ws(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut prev_space = false;
    for ch in source.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

fn push_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn match_keyword_at(upper: &str, idx: usize) -> Option<(&'static str, usize)> {
    let chars: Vec<char> = upper.chars().collect();
    if idx >= chars.len() {
        return None;
    }

    // Longest match first.
    const KEYWORDS: &[&str] = &[
        "LEFT OUTER JOIN",
        "RIGHT OUTER JOIN",
        "FULL OUTER JOIN",
        "LEFT JOIN",
        "RIGHT JOIN",
        "INNER JOIN",
        "OUTER JOIN",
        "FULL JOIN",
        "CROSS JOIN",
        "UNION ALL",
        "GROUP BY",
        "ORDER BY",
        "SELECT",
        "INSERT",
        "UPDATE",
        "DELETE",
        "VALUES",
        "RETURNING",
        "INTERSECT",
        "EXCEPT",
        "HAVING",
        "OFFSET",
        "LIMIT",
        "WHERE",
        "UNION",
        "FROM",
        "JOIN",
        "WITH",
        "SET",
        "ON",
    ];

    for kw in KEYWORDS {
        let kw_chars: Vec<char> = kw.chars().collect();
        let len = kw_chars.len();
        if idx + len > chars.len() {
            continue;
        }
        if chars[idx..idx + len] != kw_chars[..] {
            continue;
        }
        let before_ok = idx == 0 || !is_ident_char(chars[idx - 1]);
        let after_ok = idx + len >= chars.len() || !is_ident_char(chars[idx + len]);
        if before_ok && after_ok {
            return Some((kw, len));
        }
    }
    None
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '#'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_select_list() {
        let src = "SELECT Id, Company, Question, Answer FROM Faq WHERE Company = 'Wealth' ORDER BY Sequence;";
        let out = format_sql_text(src);
        assert!(out.contains("SELECT\n"));
        assert!(out.contains("  Id,\n  Company,\n"));
        assert!(out.contains("\nFROM Faq"));
        assert!(out.contains("\nWHERE Company = 'Wealth'"));
        assert!(out.contains("\nORDER BY Sequence;"));
        assert!(out.contains("'Wealth'"));
    }

    #[test]
    fn preserves_expressions() {
        let src = "SELECT * FROM t WHERE id = {{ $json.id }}";
        let out = format_sql_text(src);
        assert!(out.contains("{{ $json.id }}"));
    }
}
