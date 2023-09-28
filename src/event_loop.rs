use crate::{
    shared_context::SharedContext,
    task::Task,
    waker::{create_waker, WakerContext},
};
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
    task::{Context, Poll},
};

// Share context for "contextless" schedule call
static SHARED_CTX: OnceCell<Arc<SharedContext>> = OnceCell::new();

pub fn schedule<F: Future<Output = ()> + Send + 'static>(fut: F) {
    SHARED_CTX
        .get()
        .expect("Event loop must be started")
        .scheduled_task(fut);
}

pub struct SingleThreadEventLoop {
    shared_ctx: Arc<SharedContext>,
    tasks: HashMap<usize, Task>,
}

impl SingleThreadEventLoop {
    pub fn new() -> Self {
        Self {
            shared_ctx: Arc::new(SharedContext::new(std::thread::current())),
            tasks: Default::default(),
        }
    }

    pub fn run<F: Future<Output = ()> + 'static + Send>(mut self, main_fut: F) {
        if SHARED_CTX.get().is_some() {
            panic!("Runtime already started");
        }

        let main_task_id = self.shared_ctx.scheduled_task(main_fut);
        self.shared_ctx.wake_task(main_task_id);

        SHARED_CTX
            .set(self.shared_ctx.clone())
            .map_err(|_| ())
            .unwrap();

        loop {
            let shared_ctx = self.shared_ctx.as_ref();

            // Take all ready task ids
            let mut ready_tasks = shared_ctx.take_waked_tasks();

            // Extend ready tasks with scheduled tasks
            self.shared_ctx
                .take_scheduled_tasks()
                .into_iter()
                .for_each(|task| {
                    ready_tasks.push(task.id);
                    self.tasks.insert(task.id, task);
                });

            // Poll each ready task
            for ready_task_id in ready_tasks {
                let waker = create_waker(WakerContext::new(self.shared_ctx.clone(), ready_task_id));
                let mut ctx = Context::from_waker(&waker);

                match self.tasks.get_mut(&ready_task_id) {
                    Some(task) => {
                        match task.fut.as_mut().poll(&mut ctx) {
                            Poll::Ready(_) => {
                                // If task is ready. It should be removed
                                self.tasks.remove(&ready_task_id);
                            }
                            Poll::Pending => {
                                // Just do nothing
                            }
                        };
                    }
                    None => {
                        // Case when task waked multiple times
                    }
                }
            }

            // Break loop if no tasks there
            if self.tasks.is_empty() {
                break;
            } else {
                // Park thread until any task woke
                std::thread::park();
            }
        }
    }
}
