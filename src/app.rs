use crate::note::{DEFAULT_FOLDER_NAME, DEFAULT_NAME, DEFAULT_ROOT_NAME, DEFAULT_TRASH_NAME, Note};
use crate::storage::{DataNode, DataType, Directory, ObjectId, Storage, as_dir, as_dir_mut};
use crate::util::generate_unique_name;

pub struct NoteLookup<'note> {
    pub node: &'note DataNode<DataType<Note>>,
    pub note: &'note Note,
    pub display_type: DisplayType,
}

impl<'x> NoteLookup<'x> {
    fn scratch_pad(node: &'x DataNode<DataType<Note>>) -> Self {
        let note = as_note(&node.data).expect("Must be note");
        Self {
            node,
            note,
            display_type: DisplayType::ScratchPad,
        }
    }

    fn default_node(node: &'x DataNode<DataType<Note>>) -> Self {
        let note = as_note(&node.data).expect("Must be note");
        Self {
            node,
            note,
            display_type: DisplayType::Default,
        }
    }

    fn deleted(node: &'x DataNode<DataType<Note>>) -> Self {
        let note = as_note(&node.data).expect("Must be note");
        Self {
            node,
            note,
            display_type: DisplayType::Deleted,
        }
    }

    fn default_or_deleted(node: &'x DataNode<DataType<Note>>) -> Self {
        if node.is_deleted() {
            Self::deleted(node)
        } else {
            Self::default_node(node)
        }
    }
}

pub enum DisplayType {
    ScratchPad,
    Deleted,
    Default,
}

pub enum ContentLookup<'note> {
    Mut(&'note mut Note),
    Immut(&'note Note),
}

impl<'note> ContentLookup<'note> {
    pub fn is_mut(&self) -> bool {
        match self {
            ContentLookup::Mut(_) => true,
            ContentLookup::Immut(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn read(&self) -> &Note {
        match self {
            ContentLookup::Mut(content) => content,
            ContentLookup::Immut(content) => content,
        }
    }
}

pub struct AppState {
    pub current_note_id: u64,
    pub scratch_pad_id: ObjectId,
    pub storage: Storage<Note>,
    pub trash_dir_id: ObjectId,
}

impl AppState {
    pub fn initial() -> Self {
        let scratch_pad = Note::scratch_pad();
        let root = Directory::with_name(DEFAULT_ROOT_NAME.to_owned());
        let trash_dir = Directory::with_name(DEFAULT_TRASH_NAME.to_owned());
        let mut tree = Storage::with_root(root);
        let trash_dir_id = tree.add_dir(tree.root_dir_id(), trash_dir);
        let scratch_pad_id = tree.add_object(scratch_pad);
        Self {
            current_note_id: scratch_pad_id,
            scratch_pad_id,
            storage: tree,
            trash_dir_id,
        }
    }

    pub fn scratch_pad_mut(&mut self) -> &mut Note {
        as_note_mut(
            &mut self
                .storage
                .get_object_mut(self.scratch_pad_id)
                .expect("Scratch pad must be in storage")
                .data,
        )
        .expect("Scratch Pad must be note")
    }

    pub fn scratch_pad(&self) -> &Note {
        as_note(
            &self
                .storage
                .get_object(self.scratch_pad_id)
                .expect("Scratch Pad must be in storage")
                .data,
        )
        .expect("Scratch Pad must be note")
    }

    pub fn lookup_current_note(&self) -> NoteLookup {
        if self.scratch_pad_id == self.current_note_id {
            NoteLookup::scratch_pad(
                self
                    .storage
                    .get_object(self.current_note_id)
                    .expect("Scratch Pad must be in storage"),
            )
        } else {
            NoteLookup::default_or_deleted(
                self
                    .storage
                    .get_object(self.current_note_id)
                    .expect("File must be in storage"),
            )
        }
    }

    pub fn lookup_current_note_content(&mut self) -> ContentLookup {
        if self.scratch_pad_id == self.current_note_id {
            ContentLookup::Mut(self.scratch_pad_mut())
        } else {
            let node = self
                .storage
                .get_object_mut(self.current_note_id)
                .expect("File must be in storage");
            let is_deleted = node.is_deleted();
            let note = as_note_mut(&mut node.data).expect("Must be note");
            if is_deleted {
                ContentLookup::Immut(note)
            } else {
                ContentLookup::Mut(note)
            }
        }
    }

    pub fn new_note_then_switch(&mut self, parent_folder_id: ObjectId) {
        let id = self.add_note_with_auto_name(parent_folder_id, DEFAULT_NAME.to_owned());
        self.current_note_id = id;
    }

    fn add_note_with_auto_name(&mut self, parent_folder_id: ObjectId, name: String) -> ObjectId {
        let note = Note::default();
        let id = self.storage.add_object(note);
        let parent_folder = self
            .storage
            .get_object_mut(parent_folder_id)
            .map(|obj| as_dir_mut(&mut obj.data).expect("Must be dir"))
            .expect("Parent dir must be in storage");
        parent_folder.add_entry_with_unique_name(id, name);
        id
    }

    fn add_folder_with_auto_name(&mut self, parent_folder_id: ObjectId, name: String) -> ObjectId {
        let siblings = self
            .storage
            .get_sub_directories(parent_folder_id)
            .expect("Parent must present in tree");
        let name = generate_unique_name(
            siblings
                .iter()
                .map(|&node| as_dir(&node.data).expect("Must be dir").name.as_str()),
            name,
        );
        let dir = Directory::with_name(name);
        
        self.storage.add_dir(parent_folder_id, dir)
    }

    pub fn new_note(&mut self, parent_folder_id: ObjectId) {
        self.add_note_with_auto_name(parent_folder_id, DEFAULT_NAME.to_owned());
    }

    pub fn new_folder(&mut self, parent_folder_id: ObjectId) {
        self.add_folder_with_auto_name(parent_folder_id, DEFAULT_FOLDER_NAME.to_owned());
    }

    pub fn touch_current_note(&mut self) {
        self.storage
            .get_object_mut(self.current_note_id)
            .expect("Current note must be present")
            .touch();
    }

    pub fn delete_dir(&mut self, dir_id: ObjectId) {
        let dir_name = self
            .storage
            .get_object_mut(dir_id)
            .map(|node| {
                as_dir_mut(&mut node.data)
                    .expect("Must be dir")
                    .name
                    .to_owned()
            })
            .expect("Must be in storage");
        self.delete_object(dir_id, dir_name);
    }

    pub fn delete_object(&mut self, id: ObjectId, object_name: String) {
        self
            .storage
            .get_object_mut(id)
            .map(|obj| obj.delete())
            .expect("Object must be in storage to delete");
        self.storage
            .get_object_mut(self.trash_dir_id)
            .map(|obj| as_dir_mut(&mut obj.data).expect("Trash must be dir"))
            .expect("Trash dir must be created")
            .entries
            .insert(object_name, id);
    }

    pub fn restore_object(&mut self, id: ObjectId) {
        self.storage
            .get_object_mut(id)
            .map(|obj| obj.restore())
            .unwrap()
    }

    pub fn get_item_path_str(&self, id: ObjectId) -> Option<String> {
        self.storage.get_object_path(id).map(|path| {
            path.iter()
                .map(|node| {
                    self.storage
                        .get_object(node.dir_id())
                        .map(|obj| as_dir(&obj.data).expect("Must be dir in path"))
                        .unwrap()
                        .name
                        .to_owned()
                })
                .collect::<Vec<String>>()
                .join("/")
        })
    }
}

pub fn as_note(data: &DataType<Note>) -> Option<&Note> {
    match data {
        DataType::File(file) => Some(file),
        _ => None,
    }
}
pub fn as_note_mut(data: &mut DataType<Note>) -> Option<&mut Note> {
    match data {
        DataType::File(file) => Some(file),
        _ => None,
    }
}
