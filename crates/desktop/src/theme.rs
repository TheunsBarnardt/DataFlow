#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub fn as_class(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}
