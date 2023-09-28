use std::{future::Future, pin::Pin};

pub struct Task {
    pub id: usize,
    pub fut: Pin<Box<dyn Future<Output = ()> + Send>>,
}
