use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    hash::Hash,
};

#[derive(Debug)]
pub struct TreeLike<Key, Value> {
    pub root: Key,
    parent_to_child: HashMap<Key, HashSet<Key>>,
    child_to_parent: HashMap<Key, Key>,
    key_to_value: HashMap<Key, Value>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeRef<'value, Key: Hash + Eq, Value> {
    pub key: Key,
    pub value: &'value Value,
    pub children: HashSet<Key>,
    pub parent: Option<Key>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeMut<'value, Key: Hash + Eq, Value> {
    pub key: Key,
    pub value: &'value mut Value,
    pub children: HashSet<Key>,
    pub parent: Option<Key>,
}

pub enum TreeLikeError {
    NodeNotFound,
}

#[allow(dead_code)]
impl<Key: Hash + Eq + Clone + Copy, Value> TreeLike<Key, Value> {
    pub fn with_root(key: Key, value: Value) -> Self {
        Self {
            root: key,
            parent_to_child: Default::default(),
            child_to_parent: Default::default(),
            key_to_value: HashMap::from_iter([(key, value)]),
        }
    }

    pub fn add_node(
        &mut self,
        parent: Key,
        key_value: impl Into<(Key, Value)>,
    ) -> Result<(), TreeLikeError> {
        // Check if parent exists in tree
        self.key_to_value
            .get(&parent)
            .ok_or(TreeLikeError::NodeNotFound)?;
        let (key, value) = key_value.into();
        // Insert key->value
        self.key_to_value.insert(key, value);
        // Insert child->parent
        self.child_to_parent.insert(key, parent);
        // Add to parent->children mapping
        match self.parent_to_child.entry(parent) {
            Entry::Vacant(parent_entry) => parent_entry.insert(HashSet::new()).insert(key),
            Entry::Occupied(parent_entry) => parent_entry.into_mut().insert(key),
        };
        Result::Ok(())
    }

    #[allow(dead_code)]
    pub fn root_add_node(
        &mut self,
        key_value: impl Into<(Key, Value)>,
    ) -> Result<(), TreeLikeError> {
        self.add_node(self.root, key_value)
    }

    pub fn add_subtree(
        &mut self,
        parent: Key,
        subtree: TreeLike<Key, Value>,
    ) -> Result<(), TreeLikeError> {
        // Check if parent exist in tree
        self.key_to_value
            .get(&parent)
            .ok_or(TreeLikeError::NodeNotFound)?;
        // Merge key->value mappings
        self.key_to_value.extend(subtree.key_to_value);
        // Merge child->parent
        self.child_to_parent.extend(subtree.child_to_parent);
        // Merge parent->child
        self.parent_to_child.extend(subtree.parent_to_child);
        // Relate parent to added subtree root
        match self.parent_to_child.entry(parent) {
            Entry::Vacant(parent_entry) => parent_entry.insert(HashSet::new()).insert(subtree.root),
            Entry::Occupied(parent_entry) => parent_entry.into_mut().insert(subtree.root),
        };
        // Relate added subtree root to parent
        self.child_to_parent.insert(subtree.root, parent);
        Result::Ok(())
    }

    pub fn get_children(&self, key: Key) -> Option<HashSet<Key>> {
        let _ = self.get_value(key)?;
        Some(
            self.parent_to_child
                .get(&key)
                .cloned()
                .unwrap_or(HashSet::new()),
        )
    }

    #[allow(dead_code)]
    pub fn get_parent(&self, key: Key) -> Option<Key> {
        self.child_to_parent.get(&key).cloned()
    }

    pub fn get_value(&self, key: Key) -> Option<&Value> {
        self.key_to_value.get(&key)
    }

    pub fn get_node(&self, key: Key) -> Option<NodeRef<Key, Value>> {
        let value = self.key_to_value.get(&key)?;
        let children = self
            .parent_to_child
            .get(&key)
            .cloned()
            .unwrap_or_else(Default::default);
        let parent = self.child_to_parent.get(&key).cloned();

        Some(NodeRef {
            key,
            value,
            children,
            parent,
        })
    }

    pub fn get_node_mut(&mut self, key: Key) -> Option<NodeMut<Key, Value>> {
        let value: &mut Value = self.key_to_value.get_mut(&key)?;
        let children = self
            .parent_to_child
            .get(&key)
            .cloned()
            .unwrap_or_else(Default::default);
        let parent = self.child_to_parent.get(&key).cloned();

        Some(NodeMut {
            key,
            value,
            children,
            parent,
        })
    }

    pub fn root_node(&self) -> NodeRef<Key, Value> {
        self.get_node(self.root).expect("Root must be present")
    }

    #[allow(dead_code)]
    pub fn root_node_mut(&mut self) -> NodeMut<Key, Value> {
        self.get_node_mut(self.root).expect("Root must be present")
    }

    pub fn remove_subtree(&mut self, key: Key) -> Option<TreeLike<Key, Value>> {
        // Remove value. If not in tree then return None
        let value = self.key_to_value.remove(&key)?;
        // Remove child->parent mapping
        let parent = self.child_to_parent.remove(&key);
        // If parent existed then remove child from parent->children mapping
        parent.map(|parent| {
            self.parent_to_child
                .get_mut(&parent)
                .map(|children| children.remove(&key))
        });
        // Remove parent->children mapping
        let children = self.parent_to_child.remove(&key);
        // Recursively remove children
        let children_removed = children
            .into_iter()
            .flatten()
            .filter_map(|child| self.remove_subtree(child))
            .collect::<Vec<TreeLike<Key, Value>>>();
        Some(Self {
            // Copy root key
            root: key,
            // Collect parent->child mappings from recursively removed children
            parent_to_child: children_removed
                .iter()
                .map(|child| child.parent_to_child.clone())
                .reduce(|acc, next| acc.into_iter().chain(next).collect())
                .unwrap_or(HashMap::new()),
            // Collect child->parent mappings from recursively removed children
            child_to_parent: children_removed
                .iter()
                .map(|child| child.child_to_parent.clone())
                .reduce(|acc, next| acc.into_iter().chain(next).collect())
                .unwrap_or(HashMap::new()),
            // Collect key->value map from recursively removed children,
            // copying keys,
            // moving values
            key_to_value: children_removed
                .into_iter()
                .flat_map(|child| child.key_to_value.into_iter())
                .chain([(key, value)])
                .collect(),
        })
    }

    #[allow(dead_code)]
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.key_to_value.values()
    }

    #[allow(dead_code)]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.key_to_value.values_mut()
    }
}

#[cfg(test)]
mod tree_like_test {
    use crate::id_gen::{IdGen, TimestampRandIdGen};

    use super::*;

    fn create_tree() -> (TreeLike<u64, String>, u64, u64, u64) {
        let mut id_gen = TimestampRandIdGen::new();
        let (root_key, root_val) = (id_gen.next(), "root".to_owned());

        let mut tree = TreeLike::with_root(root_key, root_val.clone());

        let (child_key, child_val) = (id_gen.next(), "child".to_owned());
        let _ = tree.add_node(root_key, (child_key, child_val.clone()));

        let (child_key1, child_val1) = (id_gen.next(), "child1".to_owned());
        let _ = tree.add_node(child_key, (child_key1, child_val1));

        (tree, root_key, child_key, child_key1)
    }

    #[test]
    fn example() {
        let mut id_gen = TimestampRandIdGen::new();
        let root_val = "root".to_owned();
        let root_key = id_gen.next();

        let mut tree = TreeLike::with_root(root_key.clone(), root_val.clone());

        let root_node = tree.get_node(root_key);

        assert_eq!(&root_val, root_node.unwrap().value);

        let child_val = "child".to_owned();
        let child_key = id_gen.next();

        let _ = tree.add_node(root_key, (child_key, child_val.clone()));
        let node = tree.get_node(child_key).unwrap();
        let parent_node = tree.get_node(node.parent.unwrap()).unwrap();

        assert_eq!(&child_val, node.value);
        assert_eq!(child_key, parent_node.children.into_iter().next().unwrap());
        assert_eq!(&root_val, parent_node.value);

        let root_node = tree.get_node(node.parent.unwrap()).unwrap();
        assert_eq!(root_node.children.into_iter().next().unwrap(), child_key);

        let child_val1 = "child1".to_owned();
        let child_key1 = id_gen.next();
        let _ = tree.add_node(child_key, (child_key1, child_val1));
    }

    #[test]
    fn values_search_by_field() {
        let (tree, ..) = create_tree();
        let result = tree.values().find(|&it| it == "child").unwrap();
        assert_eq!("child", result);
    }
}
