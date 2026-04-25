use std::sync::{Arc, OnceLock};

use crate::event::{DispatchDescriptor, Event};

#[derive(Debug)]
pub struct AsyncRuntime {
    pub executor: smol::Executor<'static>,
    pub tx: async_channel::Sender<(Event, DispatchDescriptor)>,
    pub rx: async_channel::Receiver<(Event, DispatchDescriptor)>,
    pub notify: event_listener::Event,
}
impl Default for AsyncRuntime {
    fn default() -> Self {
        let executor = smol::Executor::new();
        let (tx, rx) = async_channel::unbounded();
        let notify = event_listener::Event::new();
        Self {
            executor,
            tx,
            rx,
            notify,
        }
    }
}

static ASYNC_RUNTIME: OnceLock<Arc<AsyncRuntime>> = OnceLock::new();
pub fn init_async_runtime(num_threads: usize) {
    ASYNC_RUNTIME
        .set(Arc::new(AsyncRuntime::default()))
        .expect("already initialized");
    for _ in 0..num_threads {
        let runtime = Arc::clone(async_runtime());
        std::thread::spawn(move || {
            smol::block_on(runtime.executor.run(smol::future::pending::<()>()))
        });
    }
}
pub fn async_runtime() -> &'static Arc<AsyncRuntime> {
    ASYNC_RUNTIME.get().expect("async runtime not initialized")
}
pub fn spawn_async(future: impl Future<Output = ()> + Send + 'static) {
    let task = async_runtime().executor.spawn(future);
    task.detach();
}
pub fn emit_event_async(event: Event, descriptor: DispatchDescriptor) {
    let runtime = async_runtime();
    let _ = runtime.tx.try_send((event, descriptor));
    runtime.notify.notify(1);
}

pub async fn sleep(duration: std::time::Duration) {
    smol::Timer::after(duration).await;
}
