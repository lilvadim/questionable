use std::{
    collections::HashMap,
    fs::{Metadata, ReadDir},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use crate::util::chrono::to_date_time_utc;

#[derive(Debug, Clone)]
pub struct DataNode<Value> {
    pub data: Value,
    pub dirty: bool,
}

impl<T> DataNode<T> {
    pub fn new(data: T) -> Self {
        Self { data, dirty: false }
    }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub creation_time: DateTime<Utc>,
    pub modification_time: DateTime<Utc>,
}

impl FileMetadata {
    pub fn from_path_metadata(metadata: Metadata) -> Self {
        let creation_time = to_date_time_utc(metadata.created().unwrap());
        let modification_time = to_date_time_utc(metadata.modified().unwrap());
        Self {
            creation_time,
            modification_time,
        }
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

#[derive(Debug, Clone)]
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
