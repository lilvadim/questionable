use std::{
    collections::HashMap,
    fs::{Metadata, ReadDir},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use crate::util::chrono::to_date_time_utc;

#[derive(Debug)]
pub struct DataNode<Value> {
    pub path: PathBuf,
    pub data: Value,
    pub creation_time: DateTime<Utc>,
    pub modification_time: DateTime<Utc>,
    pub deleted_metadata: Option<DeletedMetadata>,
    pub dirty: bool,
}

impl<V> DataNode<V> {
    pub fn from_path_metadata(path: PathBuf, metadata: Metadata, value: V) -> Self {
        let creation_time = to_date_time_utc(metadata.created().unwrap());
        let modification_time = to_date_time_utc(metadata.modified().unwrap());
        Self {
            path,
            data: value,
            creation_time,
            modification_time,
            deleted_metadata: None,
            dirty: false,
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_metadata.is_some()
    }
}

#[derive(Debug)]
pub enum DirEntry {
    Dir(PathBuf),
    File(PathBuf),
}

impl DirEntry {
    pub fn path(&self) -> &Path {
        match self {
            DirEntry::Dir(path) => path,
            DirEntry::File(path) => path,
        }
    }
}

#[derive(Debug)]
pub struct Directory {
    pub entries: HashMap<String, DirEntry>,
}

impl Default for Directory {
    fn default() -> Self {
        Self {
            entries: Default::default(),
        }
    }
}

impl Directory {
    pub fn from_read_dir(read_dir: ReadDir) -> Self {
        let fs_entries = read_dir
            .flatten()
            .map(|dirent| {
                let path = dirent.path();
                if path.is_dir() {
                    DirEntry::Dir(path)
                } else if path.is_file() {
                    DirEntry::File(path)
                } else {
                    unreachable!()
                }
            })
            .map(|entry| {
                (
                    entry
                        .path()
                        .file_name()
                        .unwrap()
                        .to_owned()
                        .into_string()
                        .unwrap(),
                    entry,
                )
            })
            .collect::<HashMap<String, DirEntry>>();

        Directory {
            entries: fs_entries,
        }
    }
}

#[derive(Debug)]
pub struct DeletedMetadata {
    pub deletion_time: DateTime<Utc>,
    pub origin_path: PathBuf,
}

impl DeletedMetadata {
    pub fn deleted_now(origin_path: PathBuf) -> Self {
        DeletedMetadata {
            deletion_time: Utc::now(),
            origin_path,
        }
    }
}
