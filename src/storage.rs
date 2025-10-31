use std::{collections::HashMap, hash::Hash};

use chrono::{DateTime, Utc};

use crate::{
    id_gen::{AtomicTimestampRandIdGen, IdGen},
    tree_like::TreeLike,
    util::generate_unique_name,
};

pub type ObjectId = u64;

#[derive(Debug)]
pub struct DataNode<Value, Key = ObjectId> {
    id: Key,
    pub data: Value,
    #[allow(dead_code)]
    pub creation_time: DateTime<Utc>,
    pub modification_time: DateTime<Utc>,
    pub deletion_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub struct DirNode<Key = ObjectId> {
    dir_obj_id: Key,
}

impl<K: Copy + Clone> DirNode<K> {
    pub fn dir_id(&self) -> K {
        self.dir_obj_id
    }
    pub fn new(dir_obj_id: K) -> Self {
        Self { dir_obj_id }
    }
}

impl<V> DataNode<V> {
    fn new(value: V) -> Self {
        let now = Utc::now();
        Self {
            id: AtomicTimestampRandIdGen::new().next(),
            data: value,
            creation_time: now,
            modification_time: now,
            deletion_time: None,
        }
    }

    pub fn touch(&mut self) {
        self.modification_time = Utc::now();
    }

    pub fn delete(&mut self) {
        self.deletion_time = Some(Utc::now());
    }

    pub fn restore(&mut self) {
        self.deletion_time = None;
    }

    pub fn is_deleted(&self) -> bool {
        self.deletion_time.is_some()
    }
}

impl<V, K: Copy + Clone> DataNode<V, K> {
    pub fn id(&self) -> K {
        self.id
    }
}

impl<V> AsRef<V> for DataNode<V> {
    fn as_ref(&self) -> &V {
        &self.data
    }
}

#[derive(Debug)]
pub struct Directory<Key = ObjectId> {
    pub name: String,
    pub entries: HashMap<String, Key>,
}

#[derive(Debug)]
pub enum DataType<V> {
    Dir(Directory),
    File(V),
}

pub fn as_dir<V>(data: &DataType<V>) -> Option<&Directory> {
    match data {
        DataType::Dir(dir) => Some(dir),
        _ => None,
    }
}
pub fn as_dir_mut<V>(data: &mut DataType<V>) -> Option<&mut Directory> {
    match data {
        DataType::Dir(dir) => Some(dir),
        _ => None,
    }
}
impl<K: Eq + Hash> Directory<K> {
    pub fn with_name(name: String) -> Self {
        Self {
            name,
            entries: HashMap::new(),
        }
    }
    pub fn get_new_unique_name(&self, new_name: String) -> String {
        generate_unique_name(self.entries.keys().map(|it| it.as_str()), new_name)
    }

    pub fn add_entry_with_unique_name(&mut self, id: K, name: String) {
        let name = self.get_new_unique_name(name);
        self.entries.insert(name, id);
    }
}

#[derive(Debug)]
pub struct Storage<V> {
    dir_tree: TreeLike<ObjectId, DirNode>,
    objects: HashMap<ObjectId, DataNode<DataType<V>>>,
}

impl<V> Storage<V> {
    pub fn with_root(root: Directory) -> Self {
        let root = DataNode::new(DataType::Dir(root));
        let root_dir_node = DirNode::new(root.id);
        Self {
            dir_tree: TreeLike::with_root(root.id, root_dir_node),
            objects: HashMap::from_iter([(root.id, root)]),
        }
    }

    pub fn root_dir(&self) -> &Directory {
        let id = self.dir_tree.root_node().value.dir_obj_id;
        self.objects
            .get(&id)
            .map(|obj| as_dir(&obj.data).expect("Root must be dir"))
            .expect("Root Dir must be in storage")
    }

    pub fn root_dir_id(&self) -> ObjectId {
        self.dir_tree.root
    }

    pub fn root_dir_mut(&mut self) -> &mut Directory {
        let id = self.dir_tree.root_node().value.dir_obj_id;
        self.objects
            .get_mut(&id)
            .map(|obj| as_dir_mut(&mut obj.data).expect("Root must be dir"))
            .expect("Root Dir must be in storage")
    }

    pub fn get_dir_parent_id(&self, dir_id: ObjectId) -> Option<ObjectId> {
        self.dir_tree
            .get_node(dir_id)
            .and_then(|node| node.parent)
    }

    pub fn get_object(&self, id: ObjectId) -> Option<&DataNode<DataType<V>>> {
        self.objects.get(&id)
    }

    pub fn get_object_mut(&mut self, id: ObjectId) -> Option<&mut DataNode<DataType<V>>> {
        self.objects.get_mut(&id)
    }

    pub fn find_file_dir(&self, file_id: ObjectId) -> Option<&DataNode<DataType<V>>> {
        self.objects
            .values()
            .filter_map(|node| match &node.data {
                DataType::Dir(dir) => dir
                    .entries
                    .values()
                    .any(|dir_entry_link| *dir_entry_link == file_id)
                    .then_some(node),
                _ => None,
            })
            .next()
    }

    pub fn find_file_dir_mut(&mut self, file_id: ObjectId) -> Option<&mut DataNode<DataType<V>>> {
        self.objects
            .values_mut()
            .filter_map(|node| match &node.data {
                DataType::Dir(dir) => dir
                    .entries
                    .values()
                    .any(|dir_entry_link| *dir_entry_link == file_id)
                    .then_some(node),
                _ => None,
            })
            .next()
    }

    pub fn add_dir(&mut self, parent_dir_id: ObjectId, dir: Directory) -> ObjectId {
        let node = DataNode::new(DataType::<V>::Dir(dir));
        let dir_node = DirNode::new(node.id);
        let id = node.id;
        self.objects.insert(id, node);
        self.dir_tree
            .add_node(parent_dir_id, (dir_node.dir_obj_id, dir_node))
            .ok()
            .expect("Parent must be in tree when add dir");
        id
    }

    pub fn add_object(&mut self, object: V) -> ObjectId {
        let node = DataNode::new(DataType::File(object));
        let id = node.id;
        self.objects.insert(node.id, node);
        id
    }

    pub fn get_object_path(&self, id: ObjectId) -> Option<Vec<&DirNode>> {
        let mut path = Vec::new();
        let parent_dir = self
            .find_file_dir(id)
            .and_then(|node| self.dir_tree.get_node(node.id))?;
        let mut node = Some(parent_dir);
        while node.is_some() {
            let unwraped = node.unwrap();
            path.push(unwraped.value);
            if let Some(parent) = unwraped.parent {
                node = self.dir_tree.get_node(parent);
            } else {
                node = None;
            }
        }
        path.reverse();
        Some(path)
    }

    pub fn dir_objects(&self, dir_id: ObjectId) -> Option<Vec<(&str, &DataNode<DataType<V>>)>> {
        self.objects
            .get(&dir_id)
            .and_then(|dir| match &dir.data {
                DataType::Dir(dir) => dir
                    .entries
                    .iter()
                    .map(|(name, id)| {
                        (
                            name.as_str(),
                            self.objects.get(id).expect("Must be in storage"),
                        )
                    })
                    .collect::<Vec<(&str, &DataNode<DataType<V>>)>>()
                    .into(),
                _ => None,
            })
    }

    pub fn get_sub_directories(&self, dir_id: ObjectId) -> Option<Vec<&DataNode<DataType<V>>>> {
        self.dir_tree.get_children(dir_id).map(|children| {
            children
                .iter()
                .map(|&id| self.objects.get(&id).expect("Child must be in tree"))
                .filter(|&n| !n.is_deleted())
                .collect()
        })
    }
}
