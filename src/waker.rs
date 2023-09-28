use crate::shared_context::SharedContext;
use std::{
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
};

pub struct WakerContext {
    task_id: usize,
    shared_ctx: Arc<SharedContext>,
}

impl WakerContext {
    pub fn new(shared_ctx: Arc<SharedContext>, task_id: usize) -> Self {
        Self {
            shared_ctx,
            task_id,
        }
    }

    fn wake_up(&self) {
        self.shared_ctx.wake_task(self.task_id);
    }
}

static WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

fn waker_clone(data: *const ()) -> RawWaker {
    // let ctx = unsafe { &*(data as *const WakerContext) };

    RawWaker::new(data, &WAKER_VTABLE)
}

fn waker_wake(data: *const ()) {
    let ctx = unsafe { &*(data as *const WakerContext) };
    ctx.wake_up();
}

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
