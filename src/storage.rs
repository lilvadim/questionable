use std::{
    collections::{HashMap, hash_map::Entry},
    hash::Hash,
};

#[derive(Debug)]
pub struct TreeLike<Key, Value> {
    pub root: Key,
    parent_to_child: HashMap<Key, Vec<Key>>,
    child_to_parent: HashMap<Key, Key>,
    key_to_value: HashMap<Key, Value>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeRef<'value, Key, Value> {
    pub key: Key,
    pub value: &'value Value,
    pub children: Vec<Key>,
    pub parent: Option<Key>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeMut<'value, Key, Value> {
    pub key: Key,
    pub value: &'value mut Value,
    pub children: Vec<Key>,
    pub parent: Option<Key>,
}

impl<Key: Hash + Eq + Clone, Value> TreeLike<Key, Value> {
    pub fn with_root(key: Key, value: Value) -> Self {
        Self {
            root: key.clone(),
            parent_to_child: Default::default(),
            child_to_parent: Default::default(),
            key_to_value: HashMap::from_iter([(key, value)]),
        }
    }

    pub fn add_node(&mut self, parent: &Key, key_value: impl Into<(Key, Value)>) -> &mut Self {
        let (key, value) = key_value.into();
        self.key_to_value.insert(key.clone(), value);
        self.child_to_parent.insert(key.clone(), parent.clone());
        match self.parent_to_child.entry(parent.clone()) {
            Entry::Vacant(entry) => entry.insert(Vec::new()).push(key),
            Entry::Occupied(entry) => entry.into_mut().push(key),
        }
        self
    }

    pub fn root_add_node(&mut self, key_value: impl Into<(Key, Value)>) {
        self.add_node(&self.root.clone(), key_value);
    }

    pub fn get_node(&self, key: &Key) -> Option<NodeRef<Key, Value>> {
        let value = self.key_to_value.get(&key)?;
        let children = self
            .parent_to_child
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Vec::new());
        let parent = self.child_to_parent.get(&key).cloned();

        Some(NodeRef {
            key: key.clone(),
            value,
            children,
            parent,
        })
    }

    pub fn get_node_mut(&mut self, key: &Key) -> Option<NodeMut<Key, Value>> {
        let value: &mut Value = self.key_to_value.get_mut(&key)?;
        let children = self
            .parent_to_child
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Vec::new());
        let parent = self.child_to_parent.get(&key).cloned();

        Some(NodeMut {
            key: key.clone(),
            value,
            children,
            parent,
        })
    }

    pub fn root_node(&self) -> NodeRef<Key, Value> {
        self.get_node(&self.root).expect("Root must be present")
    }

    pub fn root_node_mut(&mut self) -> NodeMut<Key, Value> {
        self.get_node_mut(&self.root.clone())
            .expect("Root must be present")
    }

    #[allow(dead_code)]
    pub fn remove_subtree(&mut self, key: &Key) -> Option<TreeLike<Key, Value>> {
        let value = self.key_to_value.remove(&key)?;
        self.child_to_parent.remove(key);
        let children = self.parent_to_child.remove(key).unwrap_or(Vec::new());
        let mut children_removed = children
            .iter()
            .map(|child| self.remove_subtree(child))
            .flatten()
            .collect::<Vec<TreeLike<Key, Value>>>();
        Some(Self {
            root: key.clone(),
            parent_to_child: children_removed
                .iter()
                .map(|child| child.parent_to_child.clone())
                .reduce(|acc, next| acc.into_iter().chain(next).collect())
                .unwrap_or(HashMap::new()),
            child_to_parent: children_removed
                .iter()
                .map(|child| child.child_to_parent.clone())
                .reduce(|acc, next| acc.into_iter().chain(next).collect())
                .unwrap_or(HashMap::new()),
            key_to_value: children_removed
                .iter_mut()
                .map(|child| {
                    let keys = child.key_to_value.keys().cloned().collect::<Vec<Key>>();
                    (keys, &mut child.key_to_value)
                })
                .map(|(keys, map)| {
                    keys.into_iter().map(|key| {
                        let value = map.remove(&key).expect("Value must be present");
                        (key, value)
                    })
                })
                .flatten()
                .chain([(key.clone(), value)])
                .collect(),
        })
    }

    pub fn node_values(&self) -> impl Iterator<Item = &Value> {
        self.key_to_value.values()
    }

    pub fn node_values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.key_to_value.values_mut()
    }
}

#[cfg(test)]
mod tree_like_test {
    use super::*;
    use crate::{note::Note, note_folder::NoteFolder};

    fn create_tree() -> (TreeLike<u64, NoteFolder>, u64, u64, u64) {
        let root_val = NoteFolder::empty("root".to_owned());
        let root_key = root_val.id;

        let mut tree = TreeLike::with_root(root_key.clone(), root_val.clone());

        let child_val = NoteFolder::empty("child".to_owned());
        let child_key = child_val.id;
        tree.add_node(&root_key, child_val.clone());

        let child_val1 = NoteFolder::empty("child1".to_owned());
        let child_key1 = child_val1.id;
        tree.add_node(&child_key, child_val1);

        (tree, root_key, child_key, child_key1)
    }

    #[test]
    fn example() {
        let root_val = NoteFolder::empty("root".to_owned());
        let root_key = root_val.id;

        let mut tree = TreeLike::with_root(root_key.clone(), root_val.clone());

        let root_node = tree.get_node(&root_key);

        assert_eq!(&root_val, root_node.unwrap().value);

        let child_val = NoteFolder::empty("child".to_owned());
        let child_key = child_val.id;

        tree.add_node(&root_key, child_val.clone());
        let node = tree.get_node(&child_val.id).unwrap();
        let parent_node = tree.get_node(&node.parent.unwrap()).unwrap();

        assert_eq!(&child_val, node.value);
        assert_eq!(&child_key, parent_node.children.first().unwrap());
        assert_eq!(&root_val, parent_node.value);

        let root_node = tree.get_node(&node.parent.unwrap()).unwrap();
        assert_eq!(root_node.children.first().unwrap(), &child_val.id);

        let child_val1 = NoteFolder::empty("child".to_owned());
        tree.add_node(&child_key, child_val1);
    }

    #[test]
    fn values_search_by_field() {
        let (tree, ..) = create_tree();
        let result = tree
            .node_values()
            .find(|folder| folder.name == "child")
            .unwrap();
        assert_eq!("child", result.name);
    }

    #[test]
    fn values_search_map_result() {
        let (mut tree, _root, child, ..) = create_tree();
        let note = Note::new();
        let note_id = note.id;
        tree.get_node_mut(&child).unwrap().value.put(note_id, note);
        let result = tree
            .node_values()
            .find_map(|folder| folder.get(note_id))
            .unwrap();
        assert_eq!(note_id, result.id);
    }
}
