//! SVG glyphs for canvas nodes — covers React `ICON_BY_TYPE` categories.

use dioxus::prelude::*;

use crate::components::icons::{Copy, FlaskConical, Globe, Pencil, Play, Trash2};

#[component]
pub fn NodeGlyph(ty: String, class: Option<String>) -> Element {
    let cls = class.unwrap_or_else(|| "size-7".into());
    match ty.as_str() {
        "manual" => rsx! { Play { class: Some(cls) } },
        "schedule" | "interval" | "dateTime" | "timeSaved" => rsx! {
            SvgIcon { class: cls, d: "M12 6v6l4 2M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z" }
        },
        "webhook" | "httpRequest" | "n8nApi" => rsx! {
            SvgIcon { class: cls, d: "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" }
        },
        "apiEndpoint" | "mcpServerTrigger" | "mcpCall" | "server" => rsx! {
            SvgIcon { class: cls, d: "M4 6h16M4 12h16M4 18h16M8 6v.01M8 12v.01M8 18v.01" }
        },
        "if" | "switch" | "git" => rsx! {
            SvgIcon { class: cls, d: "M6 3v12M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM15 6a9 9 0 0 0-9 9M18 15v6M21 18h-6" }
        },
        "filter" | "removeDuplicates" => rsx! {
            SvgIcon { class: cls, d: "M22 3H2l8 9.46V19l4 2v-8.54L22 3z" }
        },
        "merge" | "compareDatasets" => rsx! {
            SvgIcon { class: cls, d: "m8 6 4-4 4 4M12 2v10.3M8 18l4 4 4-4M12 22V11.7" }
        },
        "code" | "function" | "html" | "executeCommand" | "ssh" => rsx! {
            SvgIcon { class: cls, d: "m16 18 6-6-6-6M8 6l-6 6 6 6" }
        },
        "set" | "editFields" | "renameKeys" => rsx! { Pencil { class: Some(cls) } },
        "mssql" | "postgres" | "mysql" | "mongoDb" | "redis" | "redisTrigger"
        | "dataTable" | "executionData" => rsx! {
            SvgIcon { class: cls, d: "M12 2C6.48 2 3 3.79 3 6v12c0 2.21 3.48 4 9 4s9-1.79 9-4V6c0-2.21-3.48-4-9-4zM3 12c0 2.21 3.48 4 9 4s9-1.79 9-4" }
        },
        "sendEmail" | "outlook" | "outlookTrigger" | "emailTriggerImap" => rsx! {
            SvgIcon { class: cls, d: "M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2zM22 6l-10 7L2 6" }
        },
        "stickyNote" => rsx! {
            SvgIcon { class: cls, d: "M15.5 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V8.5L15.5 3zM15 3v6h6" }
        },
        "wait" | "hourglass" => rsx! {
            SvgIcon { class: cls, d: "M5 22h14M5 2h14M17 22v-4.17a2 2 0 0 0-.59-1.42L12 12l-4.41 4.41A2 2 0 0 0 7 17.83V22M7 2v4.17a2 2 0 0 0 .59 1.42L12 12l4.41-4.41A2 2 0 0 0 17 6.17V2" }
        },
        "stopAndError" | "errorTrigger" | "ban" => rsx! {
            SvgIcon { class: cls, d: "M18.36 6.64A9 9 0 1 1 5.64 18.36M18.36 6.64 5.64 18.36" }
        },
        "readFile" | "csv" | "json" | "xml" | "markdown" | "readPdf" => rsx! {
            SvgIcon { class: cls, d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M16 13H8M16 17H8M10 9H8" }
        },
        "writeFile" | "filePlus" => rsx! {
            SvgIcon { class: cls, d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M12 18v-6M9 15h6" }
        },
        "listFiles" | "itemLists" => rsx! {
            SvgIcon { class: cls, d: "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" }
        },
        "copyFile" => rsx! { Copy { class: Some(cls) } },
        "deleteFile" => rsx! { Trash2 { class: Some(cls) } },
        "splitInBatches" | "splitOut" => rsx! {
            SvgIcon { class: cls, d: "M16 3h5v5M8 3H3v5M12 22v-8.3M4.93 19.07 12 12l7.07 7.07" }
        },
        "sort" => rsx! {
            SvgIcon { class: cls, d: "m3 16 4 4 4-4M7 20V4M21 8l-4-4-4 4M17 4v16" }
        },
        "aggregate" | "summarize" | "layers" => rsx! {
            SvgIcon { class: cls, d: "m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83zM2 12l8.58 3.91a2 2 0 0 0 1.66 0L22 12M2 17l8.58 3.91a2 2 0 0 0 1.66 0L22 17" }
        },
        "simulate" | "simulateTrigger" | "evaluation" | "evaluationTrigger" | "debugHelper" => {
            rsx! { FlaskConical { class: Some(cls) } }
        }
        "form" | "formTrigger" => rsx! {
            SvgIcon { class: cls, d: "M12 20h9M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" }
        },
        "teamsTrigger" | "users" | "microsoftGraph" | "dynamics" => rsx! {
            SvgIcon { class: cls, d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" }
        },
        "sharepoint" | "share" => rsx! {
            SvgIcon { class: cls, d: "M18 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 22a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM8.59 13.51l6.83 3.98M15.41 6.51l-6.82 3.98" }
        },
        "graphql" | "braces" | "aiTransform" => rsx! {
            SvgIcon { class: cls, d: "M8 3H7a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2 2 2 0 0 1 2 2v5c0 1.1.9 2 2 2h1M16 3h1a2 2 0 0 1 2 2v5a2 2 0 0 0 2 2 2 2 0 0 0-2 2v5a2 2 0 0 1-2 2h-1" }
        },
        "jwt" | "totp" | "crypto" | "key" => rsx! {
            SvgIcon { class: cls, d: "M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.78 7.78 5.5 5.5 0 0 1 7.78-7.78zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" }
        },
        "ftp" | "hardDrive" => rsx! {
            SvgIcon { class: cls, d: "M22 12H2M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11zM6 16h.01M10 16h.01" }
        },
        "zip" | "compression" | "archive" => rsx! {
            SvgIcon { class: cls, d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2zM12 11v6M10 13h4" }
        },
        "rssRead" | "rssFeedTrigger" => rsx! {
            SvgIcon { class: cls, d: "M4 11a9 9 0 0 1 9 9M4 4a16 16 0 0 1 16 16M5 19a1 1 0 1 0 0-2 1 1 0 0 0 0 2z" }
        },
        "localFileTrigger" | "folder" | "moveFile" => rsx! {
            SvgIcon { class: cls, d: "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
        },
        "executeWorkflow" | "executeWorkflowTrigger" | "workflowTrigger" => rsx! {
            SvgIcon { class: cls, d: "M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" }
        },
        "webhookRespond" | "apiRespond" => rsx! {
            SvgIcon { class: cls, d: "m22 2-7 20-4-9-9-4Z M22 2 11 13" }
        },
        "n8nTrigger" | "activationTrigger" => rsx! {
            SvgIcon { class: cls, d: "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z" }
        },
        "icalendar" => rsx! {
            SvgIcon { class: cls, d: "M8 2v4M16 2v4M3 10h18M5 4h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z" }
        },
        "convertToFile" | "extractFromFile" | "fileOutput" | "fileInput" => rsx! {
            SvgIcon { class: cls, d: "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z M14 2v4a2 2 0 0 0 2 2h4M12 18v-6M9 15h6" }
        },
        "moveBinaryData" => rsx! {
            SvgIcon { class: cls, d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M4 16l6-6 6 6" }
        },
        "htmlExtract" => rsx! {
            SvgIcon { class: cls, d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M10 13l-2 5 3-2 2 5" }
        },
        "clipboardCheck" => rsx! {
            SvgIcon { class: cls, d: "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2M9 2h6v4H9zM9 14l2 2 4-4" }
        },
        "noOp" | "circle" => rsx! {
            SvgIcon { class: cls, d: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z" }
        },
        "whatsappTrigger" | "message" => rsx! {
            SvgIcon { class: cls, d: "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" }
        },
        "rabbitmqTrigger" => rsx! {
            SvgIcon { class: cls, d: "M13 16a3 3 0 0 1 0 6H4a3 3 0 0 1 0-6zM13 16V8a3 3 0 0 0-3-3H7M20 8h-3a2 2 0 0 0-2 2v1" }
        },
        "sseTrigger" | "radio" => rsx! {
            SvgIcon { class: cls, d: "M4.9 19.1C1 15.2 1 8.8 4.9 4.9M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5M19.1 4.9C23 8.8 23 15.1 19.1 19M12 12h.01" }
        },
        "editImage" | "image" => rsx! {
            SvgIcon { class: cls, d: "M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zM9 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM21 15l-5-5L5 21" }
        },
        "excel" | "excelOnline" | "sheet" => rsx! {
            SvgIcon { class: cls, d: "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M8 13h2M8 17h2M14 13h2M14 17h2" }
        },
        "limit" | "hash" => rsx! {
            SvgIcon { class: cls, d: "M4 9h16M4 15h16M10 3 8 21M16 3l-2 18" }
        },
        "graphql_alt" | "globe" => rsx! { Globe { class: Some(cls) } },
        _ => rsx! {
            SvgIcon { class: cls, d: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z" }
        },
    }
}

#[component]
fn SvgIcon(class: String, d: String) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            class: "{class}",
            path { d: "{d}" }
        }
    }
}
