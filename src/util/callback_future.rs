use std::sync::*;
use std::task::*;

struct CallbackFutureState<Output> {
	result: Option<Output>,
	waker: Option<Waker>,
}

pub struct CallbackFuture<Output> {
	state: Arc<Mutex<CallbackFutureState<Output>>>,
}

impl<Output> CallbackFuture<Output> {
	pub fn new() -> (Self, impl FnOnce(Output)) {
		let state = std::sync::Arc::new(std::sync::Mutex::new(CallbackFutureState {
			result: Default::default(),
			waker: Default::default(),
		}));
		let callback = {
			let state = state.clone();
			move |result| {
				let mut state = state.lock().unwrap();
				state.result = Some(result);
				state.waker.take().map(&std::task::Waker::wake);
			}
		};
		(CallbackFuture { state }, callback)
	}
}

impl<Output> std::future::Future for CallbackFuture<Output> {
	type Output = Output;
	fn poll(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		let mut state = self.state.lock().unwrap();
		if let Some(result) = state.result.take() {
			std::task::Poll::Ready(result)
		} else {
			state.waker = Some(cx.waker().clone());
			std::task::Poll::Pending
		}
	}
}
