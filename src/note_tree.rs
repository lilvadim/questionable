use crate::{note::Note, note_folder::NoteFolder, storage::TreeLike};

pub struct NoteFolderTree {
    tree: TreeLike<u64, NoteFolder>,
}

impl NoteFolderTree {
    pub fn with_items(notes: impl IntoIterator<Item = Note>) -> Self {
        let root_folder = NoteFolder::with_items("_root".to_owned(), notes);
        Self {
            tree: TreeLike::with_root(root_folder.id, root_folder),
        }
    }

    pub fn root_folder(&self) -> &NoteFolder {
        self.tree.root_node().value
    }

    pub fn root_folder_mut(&mut self) -> &mut NoteFolder {
        self.tree.root_node_mut().value
    }

    pub fn get_folder(&self, id: &u64) -> Option<&NoteFolder> {
        self.tree.get_node(id).map(|node_ref| node_ref.value)
    }

    pub fn find_note_folder(&self, note_id: &u64) -> Option<&NoteFolder> {
        self.tree
            .node_values()
            .find(|note_folder| note_folder.items.contains_key(note_id))
    }

    pub fn add_folder(&mut self, parent_folder_id: &u64, folder_name: String) -> u64 {
        let folder = NoteFolder::empty(folder_name);
        let (folder_id, folder) = (folder.id, folder);
        self.tree.add_node(parent_folder_id, folder);
        folder_id
    }

    pub fn get_sub_folders(&self, folder_id: &u64) -> Option<impl Iterator<Item = &NoteFolder>> {
        let children_ids = self.tree.get_node(folder_id)?.children;
        let sub_folders = children_ids
            .into_iter()
            .map(|id| self.tree.get_node(&id))
            .flatten()
            .map(|node_ref| node_ref.value);

        Some(sub_folders)
    }
}
