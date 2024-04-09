use std::{
    future::Future, pin::Pin, sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    }, task::{Context, Poll, Wake, Waker}, thread::JoinHandle
};
pub struct Spawner;

impl Spawner {
    pub fn spawn<F: Future<Output = T> + Unpin + Send + 'static, T: Send + 'static>(
        f: F,
    ) -> JoinHandle<Result<T, ()>> {
        let (tx, rx) = channel::<Arc<RawTaks>>();
        let task = Task {
            fut: Box::new(f),
            result: None,
        };
        let raw_task = Arc::new(RawTaks {
            fut: Mutex::new(Some(Box::new(task))),
            sender: tx.clone(),
        });
        tx.send(raw_task).unwrap();
        std::thread::spawn(move || {
            while let Ok(task) = rx.recv() {
                let mut fut_guard = task.fut.lock().unwrap();
                if let Some(mut fut) = fut_guard.take() {
                    let waker = Waker::from(task.clone());
                    let mut ctx = Context::from_waker(&waker);
					let mut p = unsafe {Pin::new_unchecked(&mut *fut)};
					let p = Pin::new(& mut p);
                    match p.poll(&mut ctx) {
                        std::task::Poll::Ready(_v) => {
                            let mut dst = unsafe { std::mem::zeroed::<T>() };
                            fut.read_task_result(&mut dst as *mut _ as *mut ());
                            return Ok(dst);
                        }
                        std::task::Poll::Pending => {
                            *fut_guard = Some(fut);
                            continue;
                        }
                    };
                }
            }
            Err(())
        })
    }
}

struct Task<T: Send> {
    fut: Box<dyn Future<Output = T> + Send + 'static + Unpin>,
    result: Option<T>,
}

impl<T: Send + 'static> Future for Task<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
		let task = unsafe {self.get_unchecked_mut()};
        match Pin::new(&mut *task.fut).poll(cx) {
            std::task::Poll::Ready(v) => {
                task.result = Some(v);
                Poll::Ready(())
            }
            std::task::Poll::Pending => Poll::Pending,
        }
    }
}

struct RawTaks {
    fut: Mutex<Option<Box<dyn AnyFuture<Output = ()> + Send  + 'static>>>,
    sender: Sender<Arc<Self>>,
}
impl Wake for RawTaks {
    fn wake(self: std::sync::Arc<Self>) {
        self.sender
            .clone()
            .send(self)
            .expect("cannot waker this task");
    }
}

trait AnyFuture: Future {
    fn read_task_result(&mut self, dst: *mut ());
}

impl<T: Send + 'static> AnyFuture for Task<T> {
    fn read_task_result(&mut self, dst: *mut ()) {
        if let Some(r) = self.result.take() {
			unsafe{
				std::ptr::write(dst.cast(), r);
			};
        }
    }
}
