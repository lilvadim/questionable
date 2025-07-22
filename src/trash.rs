use std::{collections::HashMap, hash::Hash};

use chrono::{DateTime, Utc};

pub struct Trashed<T> {
    pub item: T,
    pub put_time: DateTime<Utc>,
}

impl<T> Trashed<T> {
    fn new(item: T) -> Self {
        Self {
            item,
            put_time: Utc::now(),
        }
    }
}

pub struct TrashBin<K, T> {
    inner: HashMap<K, Trashed<T>>,
}

impl<K: Hash + Eq, T> Default for TrashBin<K, T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<K: Hash + Eq, T> TrashBin<K, T> {
    pub fn put(&mut self, id: K, item: T) {
        self.inner.insert(id, Trashed::new(item));
    }

    pub fn remove(&mut self, id: &K) -> Option<T> {
        self.inner.remove(id).map(|Trashed { item, .. }| item)
    }
    
    pub fn get(&self, id: &K) -> Option<&Trashed<T>> {
        self.inner.get(id)
    }

    pub fn values(&self) -> impl Iterator<Item = &Trashed<T>> {
        self.inner.values()
    }
}
