//! Studio CSS is inlined so the desktop WebView always gets styles
//! (asset:// stylesheet links are unreliable on Windows WebView2).

pub const STUDIO_CSS: &str = concat!(
    include_str!("../assets/tailwind.css"),
    "\n",
    include_str!("../assets/studio_extras.css"),
);
