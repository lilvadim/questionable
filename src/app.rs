use std::collections::HashMap;
use std::fs::{self};
use std::io;
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use crate::data::{DataNode, Directory};
use crate::note::{Note, SCRATCH_PAD_NAME};
use crate::thread_pool::ThreadPoolExecutor;

#[derive(Debug, Default)]
pub struct FileStorage {
    pub dirs: HashMap<PathBuf, DataNode<Directory>>,
    pub notes: HashMap<PathBuf, DataNode<Note>>,
}

#[derive(Debug)]
pub struct ApplicationState {
    pub storage: FileStorage,
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

        let scratch_pad_path: Rc<Path> = Rc::from(base_path.join(format!(".{SCRATCH_PAD_NAME}")));

        Self {
            base_path,
            scratch_pad_path,
        }
    }
}

#[derive(Debug)]
pub struct PollableTask<Out> {
    rx: Receiver<Out>,
}

pub enum PollState<Out> {
    Pending,
    Completed(Out),
}

impl<Out> PollableTask<Out> {
    pub fn new(rx: Receiver<Out>) -> Self {
        Self { rx }
    }

    pub fn poll(&self) -> PollState<Out> {
        match self.rx.try_recv() {
            Ok(value) => PollState::Completed(value),
            Err(TryRecvError::Empty) => PollState::Pending,
            Err(TryRecvError::Disconnected) => panic!("Should not be called"),
        }
    }
}

#[derive(Debug, Default)]
struct BackgroundTasks {
    read_note: HashMap<PathBuf, PollableTask<io::Result<DataNode<Note>>>>,
    read_dir: HashMap<PathBuf, PollableTask<io::Result<DataNode<Directory>>>>,
}

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
                storage: Default::default(),
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
        Ok(DataNode::from_path_metadata(
            path.to_owned(),
            fs::metadata(path)?,
            Directory::from_read_dir(fs::read_dir(path)?),
        ))
    }

    fn load_note(path: &Path) -> io::Result<DataNode<Note>> {
        Ok(DataNode::from_path_metadata(
            path.to_owned(),
            fs::metadata(path)?,
            Note::from_text(fs::read_to_string(path)?),
        ))
    }

    fn save_note(note: &DataNode<Note>) -> io::Result<()> {
        fs::write(&note.path, &note.data.text)
    }

    pub fn get_note(&self, path: &Path) -> Option<&DataNode<Note>> {
        self.state.storage.notes.get(path)
    }

    pub fn scratch_pad(&self) -> Option<&DataNode<Note>> {
        self.get_note(&self.state.config.scratch_pad_path)
    }

    pub fn scratch_pad_mut(&mut self) -> Option<&mut DataNode<Note>> {
        self.get_note_mut(&self.state.config.scratch_pad_path.clone())
    }

    pub fn get_note_mut(&mut self, path: &Path) -> Option<&mut DataNode<Note>> {
        self.state.storage.notes.get_mut(path)
    }

    pub fn get_dir(&self, path: &Path) -> Option<&DataNode<Directory>> {
        self.state.storage.dirs.get(path)
    }

    pub fn poll_background_tasks(&mut self) {
        self.poll_dir_load();
        self.poll_notes_load();
    }

    pub fn poll_notes_load(&mut self) {
        let mut completed = vec![];
        self.background_tasks
            .read_note
            .iter()
            .for_each(|(path, task)| match task.poll() {
                PollState::Completed(result) => match result {
                    Ok(note) => {
                        self.state.storage.notes.insert(path.to_owned(), note);
                        completed.push(path.to_owned());
                    }
                    Err(error) => {
                        self.error_msg.push(error.to_string());
                        completed.push(path.to_owned());
                    }
                },
                _ => {}
            });
        completed.into_iter().for_each(|path| {
            self.background_tasks.read_note.remove(&path);
        });
    }

    pub fn poll_dir_load(&mut self) {
        let mut completed = vec![];
        self.background_tasks
            .read_dir
            .iter()
            .for_each(|(path, task)| match task.poll() {
                PollState::Completed(result) => match result {
                    Ok(note) => {
                        self.state.storage.dirs.insert(path.to_owned(), note);
                        completed.push(path.to_owned());
                    }
                    Err(error) => {
                        self.error_msg.push(error.to_string());
                        completed.push(path.to_owned());
                    }
                },
                _ => {}
            });
        completed.into_iter().for_each(|path| {
            self.background_tasks.read_dir.remove(&path);
        });
    }

    pub fn read_note_in_background(&mut self, path: &Path) {
        let (note_load_tx, note_load_rx) = channel();
        let path_clone = path.to_path_buf();

        self.executor.execute(move || {
            let load_result = Self::load_note(&path_clone);
            note_load_tx.send(load_result).unwrap();
        });

        self.background_tasks
            .read_note
            .insert(path.to_path_buf(), PollableTask::new(note_load_rx));
    }

    pub fn read_dir_in_background(&mut self, path: &Path) {
        let (dir_load_tx, dir_load_rx) = channel();
        let path_clone = path.to_path_buf();

        self.executor.execute(move || {
            let load_result = Self::load_dir(&path_clone);
            dir_load_tx.send(load_result).unwrap();
        });

        self.background_tasks
            .read_dir
            .insert(path.to_path_buf(), PollableTask::new(dir_load_rx));
    }

    pub fn is_note_pending(&self, path: &Path) -> bool {
        self.background_tasks.read_note.contains_key(path)
    }

    pub fn is_dir_pending(&self, path: &Path) -> bool {
        self.background_tasks.read_dir.contains_key(path)
    }

    pub fn is_note_in_storage(&self, path: &Path) -> bool {
        self.state.storage.notes.contains_key(path)
    }

    pub fn is_dir_in_storage(&self, path: &Path) -> bool {
        self.state.storage.dirs.contains_key(path)
    }

    pub fn is_selected(&self, path: &Path) -> bool {
        self.state.current_note_path == path
    }
}
