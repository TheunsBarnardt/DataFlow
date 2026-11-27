//! Simple SVG icon components mirroring lucide-style glyphs used in the React studio.

use dioxus::prelude::*;

#[component]
pub fn LayoutDashboard(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            rect { x: "3", y: "3", width: "7", height: "9", rx: "1" }
            rect { x: "14", y: "3", width: "7", height: "5", rx: "1" }
            rect { x: "14", y: "12", width: "7", height: "9", rx: "1" }
            rect { x: "3", y: "16", width: "7", height: "5", rx: "1" }
        }
    }
}

#[component]
pub fn Folder(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" }
        }
    }
}

#[component]
pub fn Plus(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M5 12h14" }
            path { d: "M12 5v14" }
        }
    }
}

#[component]
pub fn Sun(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            circle { cx: "12", cy: "12", r: "4" }
            path { d: "M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" }
        }
    }
}

#[component]
pub fn Moon(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" }
        }
    }
}

#[component]
pub fn Settings(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}

#[component]
pub fn LogOut(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" }
            polyline { points: "16 17 21 12 16 7" }
            line { x1: "21", y1: "12", x2: "9", y2: "12" }
        }
    }
}

#[component]
pub fn ArrowLeft(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M19 12H5" }
            path { d: "m12 19-7-7 7-7" }
        }
    }
}

#[component]
pub fn ChevronLeft(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "m15 18-6-6 6-6" }
        }
    }
}

#[component]
pub fn ChevronRight(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "m9 18 6-6-6-6" }
        }
    }
}

#[component]
pub fn Variable(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M8 21s-4-3-4-9 4-9 4-9" }
            path { d: "M16 3s4 3 4 9-4 9-4 9" }
            line { x1: "15", y1: "9", x2: "9", y2: "15" }
            line { x1: "9", y1: "9", x2: "15", y2: "15" }
        }
    }
}

#[component]
pub fn Key(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "m21 2-2 2" }
            path { d: "M19 4 10.5 12.5" }
            circle { cx: "7.5", cy: "15.5", r: "5.5" }
        }
    }
}

#[component]
pub fn Boxes(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M2.97 12.92A2 2 0 0 0 2 14.63v3.24a2 2 0 0 0 .97 1.71l3 1.8a2 2 0 0 0 2.06 0L12 19v-5.5l-5-3-4.03 2.42Z" }
            path { d: "M12 13.5V19l3.97 2.38a2 2 0 0 0 2.06 0l3-1.8a2 2 0 0 0 .97-1.71v-3.24a2 2 0 0 0-.97-1.71L17 10.5l-5 3Z" }
            path { d: "M7.97 4.42A2 2 0 0 0 7 6.13v4.37l5 3 5-3V6.13a2 2 0 0 0-.97-1.71l-3-1.8a2 2 0 0 0-2.06 0l-3 1.8Z" }
        }
    }
}

#[component]
pub fn Play(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            polygon { points: "6 3 20 12 6 21 6 3" }
        }
    }
}

#[component]
pub fn Chart(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M3 3v16a2 2 0 0 0 2 2h16" }
            path { d: "M7 16h8" }
            path { d: "M7 11h12" }
            path { d: "M7 6h3" }
        }
    }
}

#[component]
pub fn ScrollText(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M15 12h-5M15 8h-5" }
            path { d: "M19 17V5a2 2 0 0 0-2-2H4" }
            path { d: "M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 0 1-2 2Z" }
        }
    }
}


#[component]
pub fn Globe(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" }
            path { d: "M2 12h20" }
        }
    }
}

#[component]
pub fn Search(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            circle { cx: "11", cy: "11", r: "8" }
            path { d: "m21 21-4.3-4.3" }
        }
    }
}

#[component]
pub fn MessageSquare(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" }
        }
    }
}

#[component]
pub fn Bot(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M12 8V4H8" }
            rect { width: "16", height: "12", x: "4", y: "8", rx: "2" }
            path { d: "M2 14h2M20 14h2M15 13v2M9 13v2" }
        }
    }
}

#[component]
pub fn Wrench(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" }
        }
    }
}

#[component]
pub fn X(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M18 6 6 18" }
            path { d: "m6 6 12 12" }
        }
    }
}

#[component]
pub fn CircleCheck(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            circle { cx: "12", cy: "12", r: "10" }
            path { d: "m9 12 2 2 4-4" }
        }
    }
}

#[component]
pub fn CircleAlert(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            circle { cx: "12", cy: "12", r: "10" }
            line { x1: "12", y1: "8", x2: "12", y2: "12" }
            line { x1: "12", y1: "16", x2: "12.01", y2: "16" }
        }
    }
}

#[component]
pub fn Zap(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z" }
        }
    }
}

#[component]
pub fn FlaskConical(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M10 2v7.527a2 2 0 0 1-.211.897L4.72 20.55a1 1 0 0 0 .9 1.45h12.76a1 1 0 0 0 .9-1.45l-5.069-10.126A2 2 0 0 1 14 9.527V2" }
            path { d: "M8.5 2h7" }
            path { d: "M7 16h10" }
        }
    }
}

#[component]
pub fn ChevronUp(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "m18 15-6-6-6 6" }
        }
    }
}

#[component]
pub fn ChevronDown(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "m6 9 6 6 6-6" }
        }
    }
}

#[component]
pub fn Check(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M20 6 9 17l-5-5" }
        }
    }
}

#[component]
pub fn MoreHorizontal(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            circle { cx: "12", cy: "12", r: "1" }
            circle { cx: "19", cy: "12", r: "1" }
            circle { cx: "5", cy: "12", r: "1" }
        }
    }
}

#[component]
pub fn History(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" }
            path { d: "M3 3v5h5" }
            path { d: "M12 7v5l4 2" }
        }
    }
}

#[component]
pub fn Copy(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            rect { x: "9", y: "9", width: "13", height: "13", rx: "2" }
            path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
        }
    }
}

#[component]
pub fn Download(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
            polyline { points: "7 10 12 15 17 10" }
            line { x1: "12", y1: "15", x2: "12", y2: "3" }
        }
    }
}

#[component]
pub fn Pencil(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" }
        }
    }
}

#[component]
pub fn Share2(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            circle { cx: "18", cy: "5", r: "3" }
            circle { cx: "6", cy: "12", r: "3" }
            circle { cx: "18", cy: "19", r: "3" }
            line { x1: "8.59", y1: "13.51", x2: "15.42", y2: "17.49" }
            line { x1: "15.41", y1: "6.51", x2: "8.59", y2: "10.49" }
        }
    }
}

#[component]
pub fn Trash2(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M3 6h18" }
            path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" }
            path { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
            line { x1: "10", y1: "11", x2: "10", y2: "17" }
            line { x1: "14", y1: "11", x2: "14", y2: "17" }
        }
    }
}

#[component]
pub fn RefreshCw(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" }
            path { d: "M21 3v5h-5" }
            path { d: "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" }
            path { d: "M8 16H3v5" }
        }
    }
}

#[component]
pub fn ArrowUp(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "m12 19V5" }
            path { d: "m5 12 7-7 7 7" }
        }
    }
}

#[component]
pub fn ArrowDown(#[props(default)] class: Option<String>) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            class: class.clone().unwrap_or_else(|| "h-4 w-4".into()),
            path { d: "M12 5v14" }
            path { d: "m19 12-7 7-7-7" }
        }
    }
}