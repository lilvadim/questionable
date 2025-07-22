use crate::{
    font_icons::phosphor,
    id_gen::{IdGen, TimestampRandIdGen},
};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub id: u64,
    pub creation_time: DateTime<Utc>,
    pub modification_time: DateTime<Utc>,
    pub path: Option<PathBuf>,
    pub content: NoteContent,
}

pub const DEFAULT_ICON: &'static str = phosphor::NOTE;
pub const DEFAULT_NAME: &'static str = "a note";

pub const SCRATCH_PAD_ICON: &'static str = phosphor::PENCIL_LINE;
pub const SCRATCH_PAD_NAME: &'static str = "Scratch Pad";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteContent {
    pub name: String,
    pub text: String,
    pub metadata: Metadata,
}

impl Default for NoteContent {
    fn default() -> Self {
        Self {
            name: String::from(DEFAULT_NAME),
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

impl NoteContent {
    fn scratch_pad() -> Self {
        Self {
            name: String::from(SCRATCH_PAD_NAME),
            metadata: Metadata {
                icon: SCRATCH_PAD_ICON.to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl Note {
    /// Creates new note with new id
    pub fn new() -> Self {
        let creation_time = Utc::now();
        Self {
            id: TimestampRandIdGen::new().next(),
            creation_time,
            modification_time: creation_time,
            path: None,
            content: Default::default(),
        }
    }

    pub fn scratch_pad() -> Self {
        Self {
            content: NoteContent::scratch_pad(),
            ..Self::with_name(SCRATCH_PAD_NAME.to_owned())
        }
    }

    /// Creates new note with new id and given name
    pub fn with_name(name: String) -> Self {
        Self {
            content: NoteContent {
                name,
                ..Default::default()
            },
            ..Self::new()
        }
    }

    pub fn touch(&mut self) {
        self.modification_time = Utc::now()
    }

    pub fn icon(&self) -> &str {
        &self.content.metadata.icon
    }
}
