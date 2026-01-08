use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::{self};
use std::io;
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::data::{DataNode, Directory, FileMetadata};
use crate::note::Note;
use crate::thread_pool::ThreadPoolExecutor;

#[derive(Debug, Default)]
pub struct FileMemory {
    pub dirs: HashMap<PathBuf, MemoryCell<DataNode<Directory>>>,
    pub notes: HashMap<PathBuf, MemoryCell<DataNode<Note>>>,
    pub metadata: HashMap<PathBuf, MemoryCell<FileMetadata>>,
}

#[derive(Debug)]
pub enum MemoryCell<T> {
    PendingRead,
    Value(T),
}

impl<T> MemoryCell<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::PendingRead => None,
        }
    }

    pub fn value_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Value(value) => Some(value),
            Self::PendingRead => None,
        }
    }
}

#[derive(Debug)]
pub struct ApplicationState {
    pub memory: FileMemory,
    pub current_note_path: PathBuf,
    pub config: NotesLocationConfig,
}

#[derive(Debug)]
pub struct NotesLocationConfig {
    pub base_path: Rc<Path>,
    pub scratch_pad_path: Rc<Path>,
}

impl Default for NotesLocationConfig {
    fn default() -> Self {
        let base_path: Rc<Path> = Rc::from(std::env::home_dir().unwrap().join("questionable"));

        let scratch_pad_path: Rc<Path> = Rc::from(base_path.join(format!(".scratchpad")));

        Self {
            base_path,
            scratch_pad_path,
        }
    }
}

#[derive(Debug, Default)]
struct BackgroundTasks {
    notes: HashMap<PathBuf, Pipe<io::Result<DataNode<Note>>>>,
    dirs: HashMap<PathBuf, Pipe<io::Result<DataNode<Directory>>>>,
}

type Pipe<T> = (Sender<T>, Receiver<T>);

#[derive(Debug)]
pub struct NonBlockingApplication {
    state: ApplicationState,
    executor: ThreadPoolExecutor,
    background_tasks: BackgroundTasks,
    error_msg: Vec<String>,
}

impl NonBlockingApplication {
    pub fn init(config: NotesLocationConfig) -> io::Result<Self> {
        fs::create_dir_all(&config.base_path)?;
        if config.scratch_pad_path.try_exists()?.not() {
            fs::write(&config.scratch_pad_path, "")?;
        }

        Ok(Self {
            state: ApplicationState {
                memory: Default::default(),
                current_note_path: config.scratch_pad_path.to_path_buf(),
                config,
            },
            executor: Default::default(),
            error_msg: Default::default(),
            background_tasks: Default::default(),
        })
    }

    pub fn pop_errors(&mut self) -> Vec<String> {
        self.error_msg.drain(..).collect()
    }

    pub fn current_note_path(&self) -> &Path {
        &self.state.current_note_path
    }

    pub fn set_current_note_path(&mut self, path: PathBuf) {
        self.state.current_note_path = path;
    }

    pub fn base_dir_path(&self) -> &Path {
        &self.state.config.base_path
    }

    pub fn scratch_pad_path(&self) -> &Path {
        &self.state.config.scratch_pad_path
    }

    pub fn base_dir(&self) -> Option<&DataNode<Directory>> {
        self.get_dir(self.base_dir_path())
    }

    fn load_dir(path: &Path) -> io::Result<DataNode<Directory>> {
        Ok(DataNode::new(Directory::from_read_dir(fs::read_dir(path)?)))
    }

    fn load_note(path: &Path) -> io::Result<DataNode<Note>> {
        Ok(DataNode::new(Note::from_text(fs::read_to_string(path)?)))
    }

    fn save_note(path: &Path, note: &DataNode<Note>) -> io::Result<DataNode<Note>> {
        fs::write(path, &note.data.text)?;
        let mut note = note.clone();
        note.dirty = false;
        Ok(note)
    }

    pub fn get_note(&self, path: &Path) -> Option<&DataNode<Note>> {
        self.state
            .memory
            .notes
            .get(path)
            .and_then(MemoryCell::value)
    }

    pub fn scratch_pad(&self) -> Option<&DataNode<Note>> {
        self.get_note(&self.state.config.scratch_pad_path)
    }

    pub fn scratch_pad_mut(&mut self) -> Option<&mut DataNode<Note>> {
        self.get_note_mut(&self.state.config.scratch_pad_path.clone())
    }

    pub fn note_is_pending(&self, path: &Path) -> bool {
        self.state
            .memory
            .notes
            .get(path)
            .and_then(MemoryCell::value)
            .is_none()
    }

    pub fn get_note_mut(&mut self, path: &Path) -> Option<&mut DataNode<Note>> {
        self.state
            .memory
            .notes
            .get_mut(path)
            .and_then(MemoryCell::value_mut)
    }

    pub fn get_dir(&self, path: &Path) -> Option<&DataNode<Directory>> {
        self.state.memory.dirs.get(path).and_then(MemoryCell::value)
    }

    pub fn poll_background_tasks(&mut self) {
        self.poll_dir_tasks();
        self.poll_notes_tasks();
    }

    pub fn poll_notes_tasks(&mut self) {
        self.background_tasks
            .notes
            .iter_mut()
            .for_each(|(path, (_tx, rx))| {
                rx.try_iter().for_each(|result| match result {
                    Ok(note) => {
                        self.state
                            .memory
                            .notes
                            .insert(path.to_path_buf(), MemoryCell::Value(note));
                    }
                    Err(err) => {
                        self.error_msg.push(err.to_string());
                    }
                })
            });
    }

    pub fn poll_dir_tasks(&mut self) {
        self.background_tasks
            .dirs
            .iter_mut()
            .for_each(|(path, (_tx, rx))| {
                rx.try_iter().for_each(|result| match result {
                    Ok(dir) => {
                        self.state
                            .memory
                            .dirs
                            .insert(path.to_path_buf(), MemoryCell::Value(dir));
                    }
                    Err(err) => {
                        self.error_msg.push(err.to_string());
                    }
                })
            });
    }

    pub fn read_note_in_background(&mut self, path: &Path) {
        if self.note_in_memory(path) {
            return;
        }

        self.state
            .memory
            .notes
            .insert(path.to_path_buf(), MemoryCell::PendingRead);

        let result_pipe = match self.background_tasks.notes.entry(path.to_owned()) {
            Entry::Vacant(entry) => entry.insert(channel()).0.clone(),
            Entry::Occupied(entry) => entry.into_mut().0.clone(),
        };

        dbg!(&path);
        self.async_execute_file_task(path, result_pipe, Self::load_note);
    }

    pub fn note_is_dirty(&self, path: &Path) -> bool {
        self.state
            .memory
            .notes
            .get(path)
            .and_then(MemoryCell::value)
            .map_or(false, |node| node.dirty)
    }

    pub fn note_in_memory(&self, path: &Path) -> bool {
        self.state.memory.notes.contains_key(path)
    }

    pub fn set_dirty(&mut self, path: &Path) {
        self.state
            .memory
            .notes
            .get_mut(path)
            .and_then(MemoryCell::value_mut)
            .map(|node| {
                node.dirty = true;
            });
    }

    pub fn save_note_in_background(&mut self, path: &Path) {
        let result_pipe = match self.background_tasks.notes.entry(path.to_owned()) {
            Entry::Vacant(entry) => entry.insert(channel()).0.clone(),
            Entry::Occupied(entry) => entry.into_mut().0.clone(),
        };

        let note = self
            .state
            .memory
            .notes
            .get(path)
            .and_then(MemoryCell::value)
            .unwrap()
            .to_owned();

        self.async_execute_file_task(path, result_pipe, move |path| Self::save_note(path, &note));
    }

    fn async_execute_file_task<T: Send + 'static>(
        &self,
        path: &Path,
        result_pipe: Sender<io::Result<T>>,
        task_fn: impl Fn(&Path) -> io::Result<T> + Send + 'static,
    ) {
        let path_clone = path.to_path_buf();
        self.executor.execute(move || {
            let parse_result = task_fn(&path_clone);
            result_pipe.send(parse_result).unwrap();
        });
    }

    pub fn read_dir_in_background(&mut self, path: &Path) {
        if self.dir_in_memory(path) {
            return;
        }

        self.state
            .memory
            .dirs
            .insert(path.to_path_buf(), MemoryCell::PendingRead);

        let result_pipe = match self.background_tasks.dirs.entry(path.to_owned()) {
            Entry::Vacant(entry) => entry.insert(channel()).0.clone(),
            Entry::Occupied(entry) => entry.into_mut().0.clone(),
        };

        self.async_execute_file_task(path, result_pipe, Self::load_dir);
    }

    pub fn dir_in_memory(&self, path: &Path) -> bool {
        self.state.memory.dirs.contains_key(path)
    }

    pub fn is_selected(&self, path: &Path) -> bool {
        self.state.current_note_path == path
    }
}
