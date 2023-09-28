mod event_loop;
mod shared_context;
mod task;
mod waker;

use event_loop::SingleThreadEventLoop;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::task::Context;
use std::task::Poll;

struct MyFut {
    i: usize,
}

impl Future for MyFut {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let waker = cx.waker().clone();

        if self.i < 1 {
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                // println!("finished");
                waker.wake();
            });

            self.get_mut().i += 1;
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

struct ThreadReadWholeFile {
    path: PathBuf,
    result: Arc<Mutex<Option<std::io::Result<Vec<u8>>>>>,
    thread_started: bool,
}

impl ThreadReadWholeFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            result: Default::default(),
            thread_started: false,
        }
    }
}

impl Future for ThreadReadWholeFile {
    type Output = std::io::Result<Vec<u8>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let _self = self.get_mut();
        let waker = cx.waker().clone();

        if !_self.thread_started {
            let path_buf = _self.path.clone();
            _self.thread_started = true;
            let result = _self.result.clone();

            std::thread::spawn(move || {
                use std::io::Read;

                let read_result = std::fs::File::open(path_buf).and_then(|mut file| {
                    let mut buf = vec![];

                    file.read_to_end(&mut buf).map(|_| buf)
                });

                let mut lock = result.lock().unwrap();
                *lock = Some(read_result);
                drop(lock);

                waker.wake();
            });

            return Poll::Pending;
        }

        let mut lock = _self.result.lock().unwrap();

        if lock.is_some() {
            Poll::Ready(lock.take().unwrap())
        } else {
            Poll::Pending
        }
    }
}

struct MultipleWakerFuture {
    counter: Arc<AtomicUsize>,
    started: bool,
}

impl Future for MultipleWakerFuture {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.started {
            let counter1 = self.counter.clone();
            let counter2 = self.counter.clone();
            let waker1 = cx.waker().clone();
            let waker2 = cx.waker().clone();

            std::thread::spawn(move || {
                counter1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                waker1.wake();
            });

            std::thread::spawn(move || {
                counter2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                waker2.wake();
            });

            self.get_mut().started = true;

            return Poll::Pending;
        }

        if self.counter.load(std::sync::atomic::Ordering::Relaxed) == 2 {
            println!("multiple wake finished");
            Poll::Ready(self.counter.load(std::sync::atomic::Ordering::Relaxed))
        } else {
            Poll::Pending
        }
    }
}

fn main() {
    SingleThreadEventLoop::new().run(async {
        event_loop::schedule(async {
            for _ in 0..2 {
                println!("iter2");
                let ff = MyFut { i: 0 };
                ff.await;
            }
        });

        let f = ThreadReadWholeFile::new(PathBuf::from_str("./Cargo.toml").unwrap());

        let res = f.await;

        println!("File read {:?}", res);

        let f2 = MultipleWakerFuture {
            counter: Default::default(),
            started: false,
        };

        println!("{:?}", f2.await);
    });
}
