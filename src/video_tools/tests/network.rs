use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(super) struct FakeNetwork {
    online: tokio::sync::watch::Sender<bool>,
    pub(super) sleeps: Mutex<Vec<Duration>>,
    pub(super) disconnect_on_retry: AtomicBool,
    pub(super) clock: tokio::sync::watch::Sender<Duration>,
    pub(super) scheduled: Mutex<Vec<Duration>>,
}
impl FakeNetwork {
    pub(super) fn new() -> Self {
        Self {
            online: tokio::sync::watch::channel(true).0,
            sleeps: Mutex::new(Vec::new()),
            disconnect_on_retry: AtomicBool::new(false),
            clock: tokio::sync::watch::channel(Duration::ZERO).0,
            scheduled: Mutex::new(Vec::new()),
        }
    }
    pub(super) fn set_online(&self, online: bool) {
        self.online.send_replace(online);
    }
}
impl NetworkEnvironment for FakeNetwork {
    fn now(&self) -> Duration {
        *self.clock.borrow()
    }
    fn online(&self) -> NetworkFuture<'_, bool> {
        Box::pin(async { *self.online.borrow() })
    }
    fn sleep(&self, duration: Duration) -> NetworkFuture<'_, ()> {
        let deadline = *self.clock.borrow() + duration;
        Box::pin(async move {
            if duration >= Duration::from_secs(60) {
                self.scheduled.lock().unwrap().push(duration);
                let mut clock = self.clock.subscribe();
                while *clock.borrow_and_update() < deadline {
                    clock.changed().await.unwrap();
                }
            } else if duration == Duration::from_secs(1) {
                let mut online = self.online.subscribe();
                while !*online.borrow_and_update() {
                    online.changed().await.unwrap();
                }
            } else {
                self.sleeps.lock().unwrap().push(duration);
                if self.disconnect_on_retry.swap(false, Ordering::SeqCst) {
                    self.set_online(false);
                }
                tokio::task::yield_now().await;
            }
        })
    }
}
