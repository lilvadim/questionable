use crate::font_icons::phosphor;

pub const DEFAULT_FOLDER_NAME: &str = "Some folder";
pub const DEFAULT_ROOT_NAME: &str = "notes";
pub const DEFAULT_TRASH_NAME: &str = "trash";

pub const DEFAULT_ICON: &str = phosphor::NOTE;
pub const DEFAULT_NAME: &str = "a note";
pub const DEFAULT_TITLE: &str = "Some new note title";

pub const SCRATCH_PAD_ICON: &str = phosphor::PENCIL_LINE;
pub const SCRATCH_PAD_NAME: &str = "Scratch Pad";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub title: String,
    pub text: String,
    pub metadata: Metadata,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            title: String::from(DEFAULT_TITLE),
            text: String::new(),
            metadata: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    icon: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            icon: DEFAULT_ICON.to_owned(),
        }
    }
}

impl Note {
    pub fn scratch_pad() -> Self {
        Self {
            title: String::from(SCRATCH_PAD_NAME),
            metadata: Metadata {
                icon: SCRATCH_PAD_ICON.to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    pub fn icon(&self) -> &str {
        &self.metadata.icon
    }
}
