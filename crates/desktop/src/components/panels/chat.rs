use dioxus::prelude::*;

use crate::components::icons::{Bot, ChevronDown, ChevronLeft, ChevronRight, History, Pencil, Wrench};
use crate::components::panels::{json_id, json_string};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonSize, ButtonVariant, ResizableHandle,
    ResizableOrientation, ResizablePanel, ResizablePanelGroup, Textarea,
};

#[derive(Clone, Debug)]
struct ChatMessage {
    id: String,
    role: String,
    content: String,
    tool_name: Option<String>,
}

#[derive(Clone, Debug)]
enum Turn {
    User { message: ChatMessage },
    Assistant {
        tools: Vec<ChatMessage>,
        message: Option<ChatMessage>,
    },
}

#[derive(Clone, Debug)]
struct HistoryGroup {
    label: String,
    items: Vec<serde_json::Value>,
}

fn parse_chat_message(v: &serde_json::Value) -> ChatMessage {
    ChatMessage {
        id: v
            .get("id")
            .map(json_string)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "msg".into()),
        role: v.get("role").map(json_string).unwrap_or_default(),
        content: v.get("content").map(json_string).unwrap_or_default(),
        tool_name: v
            .get("toolName")
            .map(json_string)
            .filter(|s| !s.is_empty()),
    }
}

fn group_turns(messages: &[serde_json::Value]) -> Vec<Turn> {
    let mut turns: Vec<Turn> = Vec::new();
    let mut tools: Vec<ChatMessage> = Vec::new();

    for v in messages {
        let message = parse_chat_message(v);
        match message.role.as_str() {
            "user" => {
                tools.clear();
                turns.push(Turn::User { message });
            }
            "tool" => tools.push(message),
            "assistant" => {
                turns.push(Turn::Assistant {
                    tools: std::mem::take(&mut tools),
                    message: Some(message),
                });
            }
            _ => {}
        }
    }

    if !tools.is_empty() {
        turns.push(Turn::Assistant {
            tools,
            message: None,
        });
    }

    turns
}

fn format_tool_title(name: Option<&str>) -> String {
    match name {
        Some(n) if !n.is_empty() => format!("Ran {n}"),
        _ => "Ran a tool".into(),
    }
}

fn format_tool_result(content: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| content.to_string()),
        Err(_) => content.to_string(),
    }
}

fn parse_iso(iso: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S%.3fZ")
                .ok()
                .map(|ndt| ndt.and_utc())
        })
}

fn relative_time(iso: &str) -> String {
    let Some(ts) = parse_iso(iso) else {
        return String::new();
    };
    let diff = chrono::Utc::now().signed_duration_since(ts);
    if diff.num_seconds() < 0 {
        return String::new();
    }
    let minutes = diff.num_minutes();
    if minutes < 1 {
        return "now".into();
    }
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    if days < 7 {
        return format!("{days}d");
    }
    format!("{}w", days / 7)
}

fn history_group_label(iso: Option<&str>) -> &'static str {
    let Some(iso) = iso else {
        return "Earlier";
    };
    let Some(ts) = parse_iso(iso) else {
        return "Earlier";
    };
    let date = ts.with_timezone(&chrono::Local).date_naive();
    let today = chrono::Local::now().date_naive();
    let yesterday = today - chrono::Days::new(1);
    if date == today {
        "Today"
    } else if date == yesterday {
        "Yesterday"
    } else {
        "Earlier"
    }
}

fn build_history_groups(history: &[serde_json::Value]) -> Vec<HistoryGroup> {
    let mut groups: Vec<HistoryGroup> = Vec::new();
    for item in history {
        let iso = item
            .get("updatedAt")
            .or_else(|| item.get("createdAt"))
            .map(json_string);
        let label = history_group_label(iso.as_deref()).to_string();
        if let Some(last) = groups.last_mut() {
            if last.label == label {
                last.items.push(item.clone());
                continue;
            }
        }
        groups.push(HistoryGroup {
            label,
            items: vec![item.clone()],
        });
    }
    groups
}

#[component]
pub fn ChatPanel(
    #[props(default)] compact: bool,
    on_workflow_changed: Option<EventHandler<()>>,
) -> Element {
    let mut thread = use_signal(|| None::<serde_json::Value>);
    let mut history = use_signal(Vec::<serde_json::Value>::new);
    let mut history_open = use_signal(|| false);
    let mut text = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);

    let refresh_history = move || {
        spawn(async move {
            match crate::api::get_json("/api/threads").await {
                Ok(v) => {
                    history.set(
                        v.get("threads")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
                Err(e) => err.set(Some(e)),
            }
        });
    };

    use_effect(move || {
        spawn(async move {
            match crate::api::get_json("/api/threads").await {
                Ok(v) => {
                    let list = v
                        .get("threads")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default();
                    history.set(list.clone());
                    if let Some(first) = list.first() {
                        let id = json_id(first);
                        if !id.is_empty() {
                            match crate::api::get_json(&format!("/api/threads/{id}")).await {
                                Ok(res) => {
                                    if let Some(t) = res.get("thread") {
                                        thread.set(Some(t.clone()));
                                    }
                                    err.set(None);
                                }
                                Err(e) => err.set(Some(e)),
                            }
                            return;
                        }
                    }
                    match crate::api::post_json("/api/threads", serde_json::json!({})).await {
                        Ok(created) => {
                            if let Some(t) = created.get("thread") {
                                thread.set(Some(t.clone()));
                            }
                            err.set(None);
                        }
                        Err(e) => err.set(Some(e)),
                    }
                }
                Err(e) => err.set(Some(e)),
            }
        });
    });

    let messages: Vec<serde_json::Value> = thread()
        .as_ref()
        .and_then(|t| t.get("messages"))
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    let turns = group_turns(&messages);
    let history_groups = build_history_groups(&history());
    let scroll_viewport_id = use_signal(|| format!("chat-msgs-{}", uuid::Uuid::new_v4()));
    let mut at_bottom = use_signal(|| true);
    let scroll_tick = messages.len() + if busy() { 10_000usize } else { 0 };
    use_effect(move || {
        let _ = scroll_tick;
        if at_bottom() {
            let id = scroll_viewport_id();
            let _ = document::eval(&format!(
                "requestAnimationFrame(function(){{var el=document.getElementById('{id}'); if(el) el.scrollTop=el.scrollHeight;}});"
            ));
        }
    });
    let active_thread_id = thread().as_ref().map(json_id).unwrap_or_default();
    let thread_title = thread()
        .as_ref()
        .and_then(|t| t.get("title"))
        .map(json_string)
        .unwrap_or_else(|| "…".into());

    let message_border = if compact {
        "rounded-md border"
    } else if history_open() {
        "rounded-r-md border border-l-0"
    } else {
        "rounded-md border"
    };

    rsx! {
        div {
            class: if compact {
                "flex h-full min-h-0 flex-col gap-2 p-0"
            } else {
                "flex h-full min-h-0 flex-col gap-3 p-3"
            },
            div {
                class: if compact {
                    "flex items-center justify-between gap-2 px-3 pt-1"
                } else {
                    "flex items-center justify-between gap-2"
                },
                div { class: "min-w-0",
                    h2 { class: "text-sm font-semibold", "Agent" }
                    p { class: "truncate text-xs text-muted-foreground", "{thread_title}" }
                }
                div { class: "flex shrink-0 items-center gap-1.5",
                    Button {
                        size: ButtonSize::Sm,
                        variant: if history_open() {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Outline
                        },
                        onclick: move |_| {
                            let next = !history_open();
                            history_open.set(next);
                            if next {
                                refresh_history();
                            }
                        },
                        if history_open() {
                            ChevronLeft { class: Some("h-4 w-4".into()) }
                        } else {
                            History { class: Some("h-4 w-4".into()) }
                        }
                        "History"
                    }
                    Button {
                        size: ButtonSize::Sm,
                        variant: ButtonVariant::Outline,
                        onclick: move |_| {
                            spawn(async move {
                                match crate::api::post_json("/api/threads", serde_json::json!({}))
                                    .await
                                {
                                    Ok(v) => {
                                        if let Some(t) = v.get("thread") {
                                            thread.set(Some(t.clone()));
                                        }
                                        text.set(String::new());
                                        err.set(None);
                                        if let Ok(h) = crate::api::get_json("/api/threads").await {
                                            history.set(
                                                h.get("threads")
                                                    .and_then(|x| x.as_array())
                                                    .cloned()
                                                    .unwrap_or_default(),
                                            );
                                        }
                                    }
                                    Err(e) => err.set(Some(e)),
                                }
                            });
                        },
                        Pencil { class: Some("h-4 w-4".into()) }
                        "New chat"
                    }
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some(if compact {
                        "mx-3 bg-destructive/10 py-1.5 text-xs".into()
                    } else {
                        "bg-destructive/10 py-1.5 text-xs".into()
                    }),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            ResizablePanelGroup {
                orientation: ResizableOrientation::Horizontal,
                initial_sizes: if history_open() {
                    vec![32.0, 68.0]
                } else {
                    vec![100.0]
                },
                panel_count: if history_open() { 2 } else { 1 },
                class: Some("min-h-0 flex-1".into()),
                if history_open() {
                    ResizablePanel {
                        panel_index: 0,
                        min_pct: 18.0,
                        max_pct: 48.0,
                        class: Some("min-h-0".into()),
                        aside {
                            class: if compact {
                                "flex h-full min-h-0 flex-col border-r bg-background/80"
                            } else {
                                "flex h-full min-h-0 flex-col rounded-l-md border border-r-0 bg-background/80"
                            },
                            div { class: "min-h-0 flex-1 overflow-y-auto px-2 py-2",
                            if history_groups.is_empty() {
                                p { class: "px-2 py-3 text-xs text-muted-foreground", "No chats yet" }
                            } else {
                                for group in history_groups {
                                    div { key: "{group.label}", class: "mb-3",
                                        div { class: "px-2 pb-1.5 text-[11px] font-medium text-muted-foreground",
                                            "{group.label}"
                                        }
                                        div { class: "flex flex-col gap-0.5",
                                            for item in group.items {
                                                {
                                                    let item_id = json_id(&item);
                                                    let title = item.get("title").map(json_string).unwrap_or_else(|| "New chat".into());
                                                    let time_iso = item
                                                        .get("updatedAt")
                                                        .or_else(|| item.get("createdAt"))
                                                        .map(json_string)
                                                        .unwrap_or_default();
                                                    let rel = relative_time(&time_iso);
                                                    let active = active_thread_id == item_id;
                                                    rsx! {
                                                        button {
                                                            key: "{item_id}",
                                                            class: if active {
                                                                "flex w-full items-center gap-2 rounded-md bg-muted px-2 py-1.5 text-left text-[13px] text-foreground transition-colors"
                                                            } else {
                                                                "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
                                                            },
                                                            onclick: move |_| {
                                                                let id = item_id.clone();
                                                                spawn(async move {
                                                                    match crate::api::get_json(&format!("/api/threads/{id}")).await {
                                                                        Ok(v) => {
                                                                            if let Some(t) = v.get("thread") {
                                                                                thread.set(Some(t.clone()));
                                                                            }
                                                                            text.set(String::new());
                                                                            err.set(None);
                                                                        }
                                                                        Err(e) => err.set(Some(e)),
                                                                    }
                                                                });
                                                            },
                                                            span {
                                                                class: if active {
                                                                    "h-1.5 w-1.5 shrink-0 rounded-full bg-sky-400"
                                                                } else {
                                                                    "h-1.5 w-1.5 shrink-0 rounded-full bg-muted-foreground/50"
                                                                },
                                                            }
                                                            span { class: "min-w-0 flex-1 truncate", "{title}" }
                                                            if !rel.is_empty() {
                                                                span { class: "shrink-0 text-[11px] text-muted-foreground", "{rel}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        }
                    }
                    ResizableHandle { handle_index: 0, container_px: 720.0 }
                }

                ResizablePanel {
                    panel_index: if history_open() { 1 } else { 0 },
                    min_pct: 40.0,
                    max_pct: 100.0,
                    class: Some("min-h-0 min-w-0".into()),
                    ResizablePanelGroup {
                        orientation: ResizableOrientation::Vertical,
                        initial_sizes: vec![72.0, 28.0],
                        class: Some(format!(
                            "h-full min-h-0 min-w-0 {}",
                            if compact { "px-3 pb-3" } else { "pt-0" }
                        )),
                        ResizablePanel {
                            panel_index: 0,
                            min_pct: 35.0,
                            max_pct: 82.0,
                            class: Some("relative min-h-0".into()),
                            div { class: "relative h-full min-h-0",
                            div {
                                id: "{scroll_viewport_id()}",
                                class: "h-full min-h-0 overflow-y-auto bg-card {message_border}",
                                onscroll: move |e| {
                                    let d = e.data();
                                    let bottom = d.scroll_top() + d.client_height() as f64
                                        >= d.scroll_height() as f64 - 48.0;
                                    at_bottom.set(bottom);
                                },
                                div { class: "flex flex-col gap-4 p-3",
                            if turns.is_empty() && !busy() {
                                div { class: "flex gap-2",
                                    div {
                                        class: "flex size-8 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground",
                                        Bot { class: Some("h-4 w-4".into()) }
                                    }
                                    div { class: "flex min-w-0 flex-1 flex-col gap-1",
                                        div { class: "text-xs font-medium text-muted-foreground", "Agent" }
                                        div {
                                            class: "rounded-lg bg-muted px-3 py-2 text-sm text-foreground",
                                            "Try: \"create webhook http flow\" or paste workflow JSON."
                                        }
                                    }
                                }
                            }

                            for turn in turns {
                                match turn {
                                    Turn::User { message } => rsx! {
                                        div {
                                            key: "{message.id}",
                                            class: "flex justify-end",
                                            div {
                                                class: "max-w-[85%] rounded-lg bg-primary/10 px-3 py-2 text-sm whitespace-pre-wrap text-foreground",
                                                "{message.content}"
                                            }
                                        }
                                    },
                                    Turn::Assistant { tools, message } => {
                                        let key = message
                                            .as_ref()
                                            .map(|m| m.id.clone())
                                            .unwrap_or_else(|| {
                                                tools.iter().map(|t| t.id.as_str()).collect::<Vec<_>>().join("-")
                                            });
                                        rsx! {
                                            div { key: "{key}", class: "flex flex-col gap-2",
                                                if !tools.is_empty() {
                                                    div { class: "flex w-full min-w-0 flex-col gap-1.5 rounded-md border bg-muted/20 p-2",
                                                        for tool in tools {
                                                            details { key: "{tool.id}", class: "w-full min-w-0 rounded-md border bg-background/90",
                                                                summary {
                                                                    class: "flex cursor-pointer list-none items-center gap-2 px-2 py-1.5 text-sm text-muted-foreground marker:content-none hover:text-foreground",
                                                                    Wrench { class: Some("h-3.5 w-3.5 shrink-0".into()) }
                                                                    span { class: "min-w-0 truncate font-medium",
                                                                        "{format_tool_title(tool.tool_name.as_deref())}"
                                                                    }
                                                                    ChevronRight { class: Some("ml-auto h-3.5 w-3.5 shrink-0 opacity-60".into()) }
                                                                }
                                                                pre {
                                                                    class: "mb-1 ml-4 overflow-x-auto border-l py-2 pr-2 pl-3 whitespace-pre-wrap text-xs text-foreground",
                                                                    "{format_tool_result(&tool.content)}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                if let Some(msg) = message {
                                                    div { class: "flex gap-2",
                                                        div {
                                                            class: "flex size-8 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground",
                                                            Bot { class: Some("h-4 w-4".into()) }
                                                        }
                                                        div { class: "flex min-w-0 flex-1 flex-col gap-1",
                                                            div { class: "text-xs font-medium text-muted-foreground", "Agent" }
                                                            div {
                                                                class: "rounded-lg bg-muted px-3 py-2 text-sm whitespace-pre-wrap text-foreground",
                                                                "{msg.content}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if busy() {
                                div {
                                    class: "flex items-center gap-2 rounded-md border px-3 py-2 text-sm text-muted-foreground",
                                    Wrench { class: Some("h-3.5 w-3.5 shrink-0".into()) }
                                    span { class: "font-medium", "Working on your request" }
                                }
                            }
                            }
                            }
                            if !at_bottom() {
                                Button {
                                    size: ButtonSize::Sm,
                                    variant: ButtonVariant::Secondary,
                                    class: Some("absolute bottom-3 right-3 z-[5] h-8 w-8 rounded-full p-0 shadow-md".into()),
                                    onclick: move |_| {
                                        at_bottom.set(true);
                                        let id = scroll_viewport_id();
                                        let _ = document::eval(&format!(
                                            "requestAnimationFrame(function(){{var el=document.getElementById('{id}'); if(el) el.scrollTop=el.scrollHeight;}});"
                                        ));
                                    },
                                    ChevronDown { class: Some("h-4 w-4".into()) }
                                }
                            }
                            }
                        }
                        ResizableHandle { handle_index: 0, container_px: 360.0 }
                        ResizablePanel {
                            panel_index: 1,
                            min_pct: 18.0,
                            max_pct: 55.0,
                            class: Some("flex min-h-0 flex-col gap-2 pt-2".into()),
                            Textarea {
                                class: Some("min-h-[4.5rem] flex-1 resize-none md:text-sm".into()),
                                placeholder: Some(
                                    "Try: \"create webhook http flow\" or paste workflow JSON".into(),
                                ),
                                value: text(),
                                disabled: busy() || thread().is_none(),
                                oninput: move |e: FormEvent| text.set(e.value()),
                                onkeydown: move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter && !e.modifiers().shift() {
                                        e.prevent_default();
                                        let Some(t) = thread() else { return };
                                        let tid = json_id(&t);
                                        let content = text().trim().to_string();
                                        if content.is_empty() || busy() {
                                            return;
                                        }
                                        at_bottom.set(true);
                                        busy.set(true);
                                        let cb = on_workflow_changed;
                                        spawn(async move {
                                            let path = format!("/api/threads/{tid}/messages");
                                            match crate::api::post_json(
                                                &path,
                                                serde_json::json!({ "content": content }),
                                            )
                                            .await
                                            {
                                                Ok(v) => {
                                                    if let Some(updated) = v.get("thread") {
                                                        thread.set(Some(updated.clone()));
                                                    }
                                                    text.set(String::new());
                                                    err.set(None);
                                                    if let Some(h) = cb.as_ref() {
                                                        h.call(());
                                                    }
                                                    if let Ok(hist) =
                                                        crate::api::get_json("/api/threads").await
                                                    {
                                                        history.set(
                                                            hist.get("threads")
                                                                .and_then(|x| x.as_array())
                                                                .cloned()
                                                                .unwrap_or_default(),
                                                        );
                                                    }
                                                }
                                                Err(e) => err.set(Some(e)),
                                            }
                                            busy.set(false);
                                        });
                                    }
                                },
                            }
                            Button {
                                class: Some("shrink-0 w-full".into()),
                                disabled: busy() || thread().is_none() || text().trim().is_empty(),
                                onclick: move |_| {
                                    let Some(t) = thread() else { return };
                                    let tid = json_id(&t);
                                    let content = text().trim().to_string();
                                    if content.is_empty() || busy() {
                                        return;
                                    }
                                    at_bottom.set(true);
                                    busy.set(true);
                                    let cb = on_workflow_changed;
                                    spawn(async move {
                                        let path = format!("/api/threads/{tid}/messages");
                                        match crate::api::post_json(
                                            &path,
                                            serde_json::json!({ "content": content }),
                                        )
                                        .await
                                        {
                                            Ok(v) => {
                                                if let Some(updated) = v.get("thread") {
                                                    thread.set(Some(updated.clone()));
                                                }
                                                text.set(String::new());
                                                err.set(None);
                                                if let Some(h) = cb.as_ref() {
                                                    h.call(());
                                                }
                                                if let Ok(hist) =
                                                    crate::api::get_json("/api/threads").await
                                                {
                                                    history.set(
                                                        hist.get("threads")
                                                            .and_then(|x| x.as_array())
                                                            .cloned()
                                                            .unwrap_or_default(),
                                                    );
                                                }
                                            }
                                            Err(e) => err.set(Some(e)),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if busy() { "Working…" } else { "Send" }
                            }
                        }
                    }
                }
            }
        }
    }
}