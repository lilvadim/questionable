use std::{
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    thread::JoinHandle,
};

type Executeable = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug)]
pub struct ThreadPoolExecutor {
    request_send: Option<Sender<Executeable>>,
    workers: Vec<JoinHandle<()>>,
}

impl Default for ThreadPoolExecutor {
    fn default() -> Self {
        Self::with_threads_cnt(std::thread::available_parallelism().unwrap().get())
    }
}

impl ThreadPoolExecutor {
    pub fn with_threads_cnt(threads_cnt: usize) -> Self {
        let (request_send, request_recv) = channel();

        let request_recv = Arc::new(Mutex::new(request_recv));

        let workers = (0..threads_cnt)
            .map(|_| {
                let request_recv_clone = Arc::clone(&request_recv);
                std::thread::spawn(move || Self::exec_loop(request_recv_clone))
            })
            .collect();

        let request_send = Some(request_send);
        Self {
            request_send,
            workers,
        }
    }

    fn exec_loop(request_recv: Arc<Mutex<Receiver<Executeable>>>) {
        loop {
            match request_recv.lock().unwrap().recv() {
                Ok(executable) => executable(),
                Err(_) => break,
            }
        }
    }

    pub fn execute(&self, task: impl FnOnce() + Send + 'static) {
        self.request_send
            .as_ref()
            .unwrap()
            .send(Box::new(task))
            .unwrap();
    }
}

impl Drop for ThreadPoolExecutor {
    fn drop(&mut self) {
        std::mem::drop(self.request_send.take());
        self.workers
            .drain(..)
            .for_each(|thread| thread.join().unwrap());
    }
}
