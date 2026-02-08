pub const DEFAULT_ICON: &str = phosphor_icons::NOTE;
pub const SCRATCH_PAD_ICON: &str = phosphor_icons::PENCIL_LINE;

pub const DEFAULT_FOLDER_NAME: &str = "Some folder";
pub const DEFAULT_ROOT_NAME: &str = "notes";
pub const DEFAULT_TRASH_NAME: &str = "trash";

pub const DEFAULT_NAME: &str = "a note";
pub const DEFAULT_TITLE: &str = "Some new note title";

pub const SCRATCH_PAD_NAME: &str = "Scratch Pad";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    pub metadata: Metadata,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            text: String::new(),
            metadata: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub icon: String,
    pub is_scratch_pad: bool,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            icon: DEFAULT_ICON.to_owned(),
            is_scratch_pad: false,
        }
    }
}

impl Note {
    pub fn scratch_pad() -> Self {
        Self {
            metadata: Metadata {
                icon: SCRATCH_PAD_ICON.to_owned(),
                is_scratch_pad: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn from_text(text: String) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }
    pub fn icon(&self) -> &str {
        &self.metadata.icon
    }

    pub fn title(&self) -> Option<&str> {
        self.text
            .split_once(char::is_whitespace)
            .map(|(head, _tail)| head)
    }

    pub fn is_scratch_pad(&self) -> bool {
        self.metadata.is_scratch_pad
    }
}
