//! Rust/UI Table primitives — https://rust-ui.com/docs/components/table

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn TableWrapper(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "overflow-auto rounded-md border max-h-96",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            "data-slot": "table-wrapper",
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn Table(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "w-full max-w-7xl text-sm caption-bottom",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        table {
            "data-slot": "table",
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn TableHeader(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "[&_tr]:border-b sticky top-0 z-10 bg-card",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        thead {
            "data-slot": "table-header",
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn TableBody(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "[&_tr:last-child]:border-0",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        tbody {
            "data-slot": "table-body",
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn TableRow(
    #[props(default)] class: Option<String>,
    #[props(default)] data_state: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "border-b transition-colors data-[state=selected]:bg-muted hover:bg-muted/50",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        tr {
            "data-slot": "table-row",
            class: "{class}",
            "data-state": data_state.as_deref(),
            {children}
        }
    }
}

#[component]
pub fn TableHead(
    #[props(default)] class: Option<String>,
    #[props(default)] title: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "h-10 px-2 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-[2px]",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        th {
            "data-slot": "table-head",
            class: "{class}",
            title: title,
            {children}
        }
    }
}

#[component]
pub fn TableCell(
    #[props(default)] class: Option<String>,
    #[props(default)] title: Option<String>,
    #[props(default)] colspan: Option<u32>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "p-4 align-middle [&:has([role=checkbox])]:pr-0 [&:has([role=checkbox])]:pl-3",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        td {
            "data-slot": "table-cell",
            class: "{class}",
            title: title,
            colspan: colspan,
            {children}
        }
    }
}

#[component]
pub fn TableFooter(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "font-medium border border-t bg-muted/50 [&>tr]:last:border-b-0",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        tfoot {
            "data-slot": "table-footer",
            class: "{class}",
            {children}
        }
    }
}

#[component]
pub fn TableCaption(#[props(default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "mt-4 text-sm text-muted-foreground",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        caption {
            "data-slot": "table-caption",
            class: "{class}",
            {children}
        }
    }
}
