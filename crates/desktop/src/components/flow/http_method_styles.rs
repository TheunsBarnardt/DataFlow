pub fn http_method_badge_class(method: &str) -> &'static str {
    match method.to_uppercase().as_str() {
        "GET" => "border-transparent bg-[#61affe] text-white",
        "POST" => "border-transparent bg-[#49cc90] text-white",
        "PUT" => "border-transparent bg-[#fca130] text-white",
        "PATCH" => "border-transparent bg-[#f39c12] text-white",
        "DELETE" => "border-transparent bg-[#f93e3e] text-white",
        "HEAD" => "border-transparent bg-[#c07a4a] text-white",
        "OPTIONS" => "border-transparent bg-[#0d5aa7] text-white",
        _ => "border-transparent bg-muted text-muted-foreground",
    }
}

pub fn is_http_method_label(label: &str) -> bool {
    matches!(
        label.trim().to_uppercase().as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    )
}

pub fn is_item_count_label(label: &str) -> bool {
    label == "1 item" || label.ends_with(" items")
}