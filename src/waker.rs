use std::{
    sync::{Arc, Mutex},
    task::{RawWaker, RawWakerVTable, Waker},
};

use crate::event_loop::SingleThreadEventLoop;

pub struct WakerContext {
    event_loop: Arc<Mutex<SingleThreadEventLoop>>,
    task_id: usize,
    cloned: usize,
}

impl WakerContext {
    pub fn new(event_loop: Arc<Mutex<SingleThreadEventLoop>>, task_id: usize) -> Self {
        Self {
            event_loop,
            task_id,
            cloned: 0,
        }
    }

    fn wake_up(&self) {
        println!("wake");
        self.event_loop
            .lock()
            .unwrap()
            .notify_task_ready(self.task_id);
    }

    // fn get_next_task_id(&self) -> usize {
    //     self.tasks_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    // }

    // fn queue_task(&self, task: Task) {
    //     let task_id = task.id;

    //     let mut tasks = self.tasks.lock().unwrap();

    //     tasks.0.insert(task_id, task);
    //     tasks.1.push_back(task_id);
    // }
}

static WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| waker_clone(data),
    |data| waker_wake(data),
    |data| waker_wake_by_ref(data),
    |data| waker_drop(data),
);

fn waker_clone(data: *const ()) -> RawWaker {
    let ctx = unsafe { &*(data as *const WakerContext) };



    RawWaker::new(data, &WAKER_VTABLE)
}

fn waker_wake(data: *const ()) {
    let ctx = unsafe { &*(data as *const WakerContext) };
    ctx.wake_up();
}

/// Функция для вызова обработчика готовности задачи по ссылке.
fn waker_wake_by_ref(data: *const ()) {
    let ctx = unsafe { &*(data as *const WakerContext) };
    ctx.wake_up();
}

fn waker_drop(_: *const ()) {}

pub fn create_waker(waker_context: WakerContext) -> Waker {
    let ctx_ptr = Box::into_raw(Box::new(waker_context)) as *const ();
    let raw_waker = RawWaker::new(ctx_ptr, &WAKER_VTABLE);

    unsafe { Waker::from_raw(raw_waker) }
}
