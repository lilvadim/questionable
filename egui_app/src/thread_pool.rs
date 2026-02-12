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
    request_tx: Option<Sender<Executeable>>,
    workers: Vec<JoinHandle<()>>,
}

impl Default for ThreadPoolExecutor {
    fn default() -> Self {
        Self::with_threads_cnt(std::thread::available_parallelism().unwrap().get())
    }
}

impl ThreadPoolExecutor {
    pub fn with_threads_cnt(threads_cnt: usize) -> Self {
        let (request_tx, request_rx) = channel();

        let request_rx = Arc::new(Mutex::new(request_rx));

        let workers = (0..threads_cnt)
            .map(|_| {
                let request_rx_clone = Arc::clone(&request_rx);
                std::thread::spawn(move || Self::exec_loop(request_rx_clone))
            })
            .collect();

        let request_tx = Some(request_tx);
        Self {
            request_tx,
            workers,
        }
    }

    fn exec_loop(request_rx: Arc<Mutex<Receiver<Executeable>>>) {
        loop {
            match request_rx.lock().unwrap().recv() {
                Ok(executable) => executable(),
                Err(_) => break,
            }
        }
    }

    pub fn execute(&self, task: impl FnOnce() + Send + 'static) {
        self.request_tx
            .as_ref()
            .unwrap()
            .send(Box::new(task))
            .unwrap();
    }
}

impl Drop for ThreadPoolExecutor {
    fn drop(&mut self) {
        std::mem::drop(self.request_tx.take());
        self.workers
            .drain(..)
            .for_each(|thread| thread.join().unwrap());
    }
}
