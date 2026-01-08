use std::sync::mpsc::{Receiver, TryRecvError};

#[derive(Debug)]
pub struct PollableTask<Out> {
    rx: Receiver<Out>,
}

pub enum TaskState<Out> {
    Pending,
    Completed(Out),
}

#[derive(Debug)]
pub enum PollError {
    ChannelDisconnected,
}

impl<Out> PollableTask<Out> {
    pub fn new(rx: Receiver<Out>) -> Self {
        Self { rx }
    }

    pub fn poll(&self) -> Result<TaskState<Out>, PollError> {
        match self.rx.try_recv() {
            Ok(value) => Ok(TaskState::Completed(value)),
            Err(TryRecvError::Empty) => Ok(TaskState::Pending),
            Err(TryRecvError::Disconnected) => Err(PollError::ChannelDisconnected),
        }
    }
}
