use std::collections::BTreeMap;

use crate::{
    id_gen::{IdGen, TimestampRandIdGen},
    note::{Note, NoteContent},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteFolder {
    pub id: u64,
    pub items: BTreeMap<u64, Note>,
    pub name: String,
}

impl NoteFolder {
    pub fn empty(name: String) -> Self {
        Self {
            id: TimestampRandIdGen::new().next(),
            items: Default::default(),
            name,
        }
    }

    pub fn with_items(name: String, items: impl IntoIterator<Item = Note>) -> Self {
        Self {
            items: BTreeMap::from_iter(items.into_iter().map(|note| (note.id, note))),
            ..Self::empty(name)
        }
    }

    pub fn insert_new_with_auto_name(&mut self, name: String) -> u64 {
        let note = self.create_note_with_auto_name(name);
        let id = note.id;
        self.items.insert(id, note);
        id
    }

    pub fn get(&self, id: u64) -> Option<&Note> {
        self.items.get(&id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Note> {
        self.items.remove(&id)
    }

    pub fn put(&mut self, id: u64, item: Note) {
        self.items.insert(id, item);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Note> {
        self.items.values()
    }

    pub fn touch_note(&mut self, id: u64) -> Option<()> {
        self.items.get_mut(&id).map(|note| note.touch())
    }

    pub fn get_mut_note_content(&mut self, id: u64) -> Option<&mut NoteContent> {
        self.items.get_mut(&id).map(|note| &mut note.content)
    }

    fn create_note_with_auto_name(&self, note_name: String) -> Note {
        let note_name = self.get_indexed_auto_note_name(&note_name);
        Note::with_name(note_name)
    }

    fn get_indexed_auto_note_name(&self, note_name: &str) -> String {
        format!("{} #{}", note_name, self.items.len())
    }
}

impl Into<(u64, NoteFolder)> for NoteFolder {
    fn into(self) -> (u64, NoteFolder) {
        (self.id, self)
    }
}
