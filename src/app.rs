use crate::note::{DEFAULT_NAME, Note, NoteContent};
use crate::note_tree::NoteFolderTree;
use crate::trash::{TrashBin, Trashed};

pub enum NoteLookup<'note> {
    Default(&'note Note),
    ScratchPad(&'note Note),
    Trashed(&'note Trashed<Note>),
}

pub enum ContentLookup<'note> {
    Mut(&'note mut NoteContent),
    Immut(&'note NoteContent),
}

impl<'note> ContentLookup<'note> {
    pub fn is_mut(&self) -> bool {
        match self {
            ContentLookup::Mut(_) => true,
            ContentLookup::Immut(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn read(&self) -> &NoteContent {
        match self {
            ContentLookup::Mut(content) => content,
            ContentLookup::Immut(content) => content,
        }
    }
}

impl<'note> NoteLookup<'note> {
    #[allow(dead_code)]
    pub fn is_trashed(&self) -> bool {
        match self {
            NoteLookup::Trashed(_) => true,
            _ => false,
        }
    }

    pub fn into_note(self) -> &'note Note {
        match self {
            NoteLookup::Default(note) => note,
            NoteLookup::ScratchPad(note) => note,
            NoteLookup::Trashed(trashed) => &trashed.item,
        }
    }
}

pub struct AppState {
    pub current_note_id: u64,
    pub scratch_pad: Note,
    pub notes: NoteFolderTree,
    pub trash: TrashBin<u64, Note>,
}

impl AppState {
    pub fn current_note(&self) -> &Note {
        self.lookup_current_note().into_note()
    }

    pub fn lookup_current_note(&self) -> NoteLookup {
        if self.scratch_pad.id == self.current_note_id {
            NoteLookup::ScratchPad(&self.scratch_pad)
        } else {
            lookup_note(self.current_note_id, &self.notes, &self.trash)
                .expect("Current note must be present (during lookup note)")
        }
    }

    pub fn lookup_current_note_content(&mut self) -> ContentLookup {
        if self.scratch_pad.id == self.current_note_id {
            ContentLookup::Mut(&mut self.scratch_pad.content)
        } else {
            lookup_note_content(self.current_note_id, &mut self.notes, &self.trash)
                .expect("Current note must be present (during lookup note content)")
        }
    }

    pub fn new_note_then_switch(&mut self) {
        let id = self
            .notes
            .root_folder_mut()
            .insert_new_with_auto_name(DEFAULT_NAME.to_owned());
        self.current_note_id = id;
    }

    pub fn new_folder(&mut self, folder_name: String) {
        let current_note_folder_id = self
            .notes
            .find_note_folder(&self.current_note_id)
            .map(|note_folder| note_folder.id)
            .unwrap_or(self.notes.root_folder().id);
        self.notes.add_folder(&current_note_folder_id, folder_name);
    }

    pub fn touch_current_note(&mut self) {
        if self.scratch_pad.id == self.current_note_id {
            self.scratch_pad.touch()
        } else {
            self.notes
                .root_folder_mut()
                .touch_note(self.current_note_id)
                .expect("Note to touch must be present")
        }
    }

    pub fn restore_note(&mut self, id: u64) {
        let restored_note: Note = self
            .trash
            .remove(&id)
            .expect("Note to restore must be present in trash");
        self.notes
            .root_folder_mut()
            .put(restored_note.id, restored_note);
    }

    pub fn trash_note(&mut self, id: u64) {
        let removed_note = self
            .notes
            .root_folder_mut()
            .remove(id)
            .expect("Note to trash must be present in storage");
        self.trash.put(removed_note.id, removed_note);
    }
}

fn lookup_note<'x>(
    id: u64,
    notes: &'x NoteFolderTree,
    trash: &'x TrashBin<u64, Note>,
) -> Option<NoteLookup<'x>> {
    notes
        .root_folder()
        .get(id)
        .map(|note| NoteLookup::Default(note))
        .or_else(|| trash.get(&id).map(|trashed| NoteLookup::Trashed(trashed)))
}

fn lookup_note_content<'x>(
    id: u64,
    notes: &'x mut NoteFolderTree,
    trash: &'x TrashBin<u64, Note>,
) -> Option<ContentLookup<'x>> {
    notes
        .root_folder_mut()
        .get_mut_note_content(id)
        .map(|content| ContentLookup::Mut(content))
        .or_else(|| {
            trash
                .get(&id)
                .map(|Trashed { item, .. }| ContentLookup::Immut(&item.content))
        })
}
