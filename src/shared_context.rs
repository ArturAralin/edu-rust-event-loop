use crate::task::Task;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread::Thread,
};

pub struct SharedContext {
    main_thread: Thread,
    scheduled_tasks: Mutex<Vec<Task>>,
    tasks_counter: AtomicUsize,
    waked_tasks: Mutex<Vec<usize>>,
}

impl SharedContext {
    pub fn new(main_thread: Thread) -> Self {
        Self {
            main_thread,
            scheduled_tasks: Default::default(),
            tasks_counter: Default::default(),
            waked_tasks: Default::default(),
        }
    }

    pub fn scheduled_task<F: Future<Output = ()> + Send + 'static>(&self, fut: F) -> usize {
        let id = self.tasks_counter.fetch_add(1, Ordering::Relaxed);

        self.scheduled_tasks.lock().unwrap().push(Task {
            id,
            fut: Box::pin(fut),
        });

        id
    }

    pub fn take_waked_tasks(&self) -> Vec<usize> {
        let mut waked_tasks = self.waked_tasks.lock().unwrap();

        std::mem::take(&mut waked_tasks)
    }

    pub fn take_scheduled_tasks(&self) -> Vec<Task> {
        let mut scheduled_tasks = self.scheduled_tasks.lock().unwrap();

        std::mem::take(&mut scheduled_tasks)
    }

    pub fn wake_task(&self, task_id: usize) {
        println!("wake_task");
        self.waked_tasks.lock().unwrap().push(task_id);
        self.main_thread.unpark();
    }
}
