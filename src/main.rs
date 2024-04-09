use std::{future::Future, sync::{atomic::{AtomicBool, Ordering}, Arc}, task::Poll};

use crate::executor::Spawner;

mod executor;

struct Timer(bool,Arc<AtomicBool>);

impl Future for Timer {
	type Output = ();

	fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
		if self.0 == false{
			self.0 = true;
			let a = self.1.clone();
			let waker = cx.waker().clone();
			std::thread::spawn(move ||{
				std::thread::sleep(std::time::Duration::from_secs(2));
				a.store(true, Ordering::Relaxed);
				waker.wake();
			});
			Poll::Pending
		}else if self.1.load(Ordering::Relaxed){
			Poll::Ready(())
		}else{
			Poll::Pending
		}
	}
}

fn main() {
    println!("Hello, world!");
	let h = Spawner::spawn(Box::pin(async {
		println!("entry task");
		Timer(false,Arc::new(AtomicBool::new(false))).await;
		println!("time 2 over");
		Timer(false,Arc::new(AtomicBool::new(false))).await;
		println!("prepare to give result");
		1
	}));
	let r = h.join().unwrap().unwrap();
	println!("{}",r);
}
