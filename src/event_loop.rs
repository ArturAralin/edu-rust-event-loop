use once_cell::sync::Lazy;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc, Mutex},
    task::{Context, Poll},
    thread::Thread,
};

use crate::waker::{create_waker, WakerContext};

static TASKS_COUNTER: Lazy<AtomicUsize> = Lazy::new(Default::default);
static SCHEDULED_TASKS: Lazy<Arc<Mutex<Vec<Task>>>> = Lazy::new(Default::default);

pub struct Task {
    id: usize,
    fut: Pin<Box<dyn Future<Output = ()> + Send>>,
}

pub struct SingleThreadEventLoop {
    main_thread: Thread,
    tasks: HashMap<usize, Task>,
    ready_tasks: VecDeque<usize>,
}

impl SingleThreadEventLoop {
    pub fn new() -> Self {
        Self {
            main_thread: std::thread::current(),
            tasks: Default::default(),
            ready_tasks: Default::default(),
        }
    }

    pub fn schedule<F: Future<Output = ()> + Send + 'static>(fut: F) {
        SCHEDULED_TASKS
            .lock()
            .unwrap()
            .push(Self::register_task(fut));
    }

    pub fn register_task<F: Future<Output = ()> + 'static + Send>(fut: F) -> Task {
        Task {
            id: TASKS_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            fut: Box::pin(fut),
        }
    }

    pub fn get_task_in_work(&mut self, task: Task) {
        let task_id = task.id;
        self.tasks.insert(task_id, task);
        self.ready_tasks.push_back(task_id);
    }

    pub fn notify_task_ready(&mut self, task_id: usize) {
        self.ready_tasks.push_back(task_id);
        self.main_thread.unpark();
    }

    pub fn run<F: Future<Output = ()> + 'static + Send>(mut self, main_fut: F) {
        if TASKS_COUNTER.load(std::sync::atomic::Ordering::SeqCst) != 0 {
            panic!("Runtime started already");
        }

        // Use atomic bool for marking ready tasks?
        let scheduled_tasks: Arc<Mutex<Vec<Task>>> = Default::default();
        let tasks_counter: Arc<AtomicUsize> = Default::default();
        let tasks: HashMap<usize, Task> = Default::default();

        self.get_task_in_work(Self::register_task(main_fut));

        let context_arc: Arc<Mutex<Self>> = Arc::new(Mutex::new(self));

        loop {
            let mut event_loop = context_arc.lock().unwrap();

            if let Some(task_id) = event_loop.ready_tasks.pop_front() {
                let waker = create_waker(WakerContext::new(context_arc.clone(), task_id));
                let mut ctx = Context::from_waker(&waker);
                // let task = .unwrap();

                match event_loop.tasks.get_mut(&task_id) {
                    Some(task) => {
                        match task.fut.as_mut().poll(&mut ctx) {
                            Poll::Pending => {}
                            Poll::Ready(_) => {
                                event_loop.tasks.remove(&task_id);
                            }
                        };
                    }
                    None => {
                        // Case when task waked multiple times
                    }
                };
            }

            let mut global_scheduled_tasks = SCHEDULED_TASKS.lock().unwrap();

            let scheduled_tasks = std::mem::replace(&mut *global_scheduled_tasks, Vec::new());

            drop(global_scheduled_tasks);

            scheduled_tasks.into_iter().for_each(|task| {
                event_loop.get_task_in_work(task);
            });

            if event_loop.tasks.is_empty() {
                break;
            }

            let ready_tasks_count = event_loop.ready_tasks.len();

            drop(event_loop);

            if ready_tasks_count == 0 {
                std::thread::park();
            }
        }
    }
}
