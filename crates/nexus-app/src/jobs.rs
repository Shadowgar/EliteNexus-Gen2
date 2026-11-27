//! Tiny background-job helper: run a closure on a worker thread, poll the result each frame.
//! Keeps the UI thread free of database queries and network calls.

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

pub struct Job<T> {
    rx: Receiver<Result<T, String>>,
    started: Instant,
}

struct Completion<T> {
    tx: Option<Sender<Result<T, String>>>,
    ctx: egui::Context,
}

impl<T> Completion<T> {
    fn send(self, value: T) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Ok(value));
        }
    }
}

impl<T> Drop for Completion<T> {
    fn drop(&mut self) {
        // Disconnect before waking the UI, including when a worker unwinds or its task is dropped.
        drop(self.tx.take());
        self.ctx.request_repaint();
    }
}

impl<T: Send + 'static> Job<T> {
    pub fn spawn(ctx: &egui::Context, f: impl FnOnce() -> T + Send + 'static) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let completion = Completion { tx: Some(tx.clone()), ctx: ctx.clone() };
        if let Err(e) = std::thread::Builder::new().spawn(move || completion.send(f())) {
            let _ = tx.send(Err(format!("Could not start the background worker: {e}")));
            ctx.request_repaint();
        }
        Job { rx, started: Instant::now() }
    }

    /// The result once finished; afterwards the job is spent (keep the value yourself).
    pub fn poll(&mut self) -> Option<T> {
        self.poll_outcome()?.ok()
    }

    /// A terminal outcome, including a worker that exited without returning a value.
    pub fn poll_outcome(&mut self) -> Option<Result<T, String>> {
        match self.rx.try_recv() {
            Ok(v) => Some(v),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("The background worker exited before returning a result.".into())),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}

impl<T: Send + 'static> Job<T> {
    /// Runs a future on the shared tokio runtime (network calls).
    pub fn spawn_async<F>(ctx: &egui::Context, rt: &tokio::runtime::Handle, fut: F) -> Self
    where
        F: std::future::Future<Output = T> + Send + 'static,
    {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let completion = Completion { tx: Some(tx), ctx: ctx.clone() };
        rt.spawn(async move {
            completion.send(fut.await);
        });
        Job { rx, started: Instant::now() }
    }
}

impl<T: Send + 'static> Slot<Result<T, String>> {
    /// Poll a fallible request and publish either its answer or a terminal worker error.
    pub fn tick_result(&mut self) -> bool {
        let Some(job) = &mut self.job else { return false };
        let Some(outcome) = job.poll_outcome() else { return false };
        self.took = Some(job.elapsed());
        self.last = Some(outcome.and_then(|result| result));
        self.job = None;
        true
    }
}

/// A job slot plus its last result and how long it took.
pub struct Slot<T> {
    pub job: Option<Job<T>>,
    pub last: Option<T>,
    pub took: Option<Duration>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Slot { job: None, last: None, took: None }
    }
}

impl<T> Slot<T> {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

impl<T: Send + 'static> Slot<T> {
    pub fn run(&mut self, ctx: &egui::Context, f: impl FnOnce() -> T + Send + 'static) {
        self.job = Some(Job::spawn(ctx, f));
    }

    pub fn run_async<F>(&mut self, ctx: &egui::Context, rt: &tokio::runtime::Handle, fut: F)
    where
        F: std::future::Future<Output = T> + Send + 'static,
    {
        self.job = Some(Job::spawn_async(ctx, rt, fut));
    }

    /// Call every frame; returns true when a new result just arrived.
    pub fn tick(&mut self) -> bool {
        if let Some(job) = self.job.as_mut()
            && let Some(v) = job.poll() {
                self.took = Some(job.elapsed());
                self.last = Some(v);
                self.job = None;
                return true;
            }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnected_worker_is_terminal_and_the_slot_can_retry() {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let mut slot = Slot { job: Some(Job { rx, started: Instant::now() }), ..Default::default() };
        assert!(!slot.tick_result());
        assert!(slot.busy());
        drop(tx);
        assert!(slot.tick_result());
        assert!(!slot.busy());
        assert!(matches!(slot.last.as_ref(), Some(Err(e)) if e.contains("exited")));
        assert!(!slot.tick_result());

        let (tx, rx) = crossbeam_channel::bounded(1);
        slot.job = Some(Job { rx, started: Instant::now() });
        tx.send(Ok(Err("query failed".into()))).unwrap();
        assert!(slot.tick_result());
        assert_eq!(slot.last.as_ref().and_then(|r| r.as_ref().err()).map(String::as_str), Some("query failed"));
        assert!(!slot.busy());

        let (tx, rx) = crossbeam_channel::bounded(1);
        slot.job = Some(Job { rx, started: Instant::now() });
        tx.send(Ok(Ok(42))).unwrap();
        assert!(slot.tick_result());
        assert_eq!(slot.last, Some(Ok(42)));
        assert!(!slot.busy());
    }

    #[test]
    fn panicked_worker_disconnects_and_repaints() {
        let ctx = egui::Context::default();
        let (notify, repaints) = crossbeam_channel::bounded(1);
        ctx.set_request_repaint_callback(move |_| {
            let _ = notify.try_send(());
        });
        let mut job = Job::<()>::spawn(&ctx, || panic!("synthetic worker panic"));
        repaints.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(job.poll_outcome(), Some(Err(_))));
    }

    #[test]
    fn cancelled_async_worker_disconnects() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let mut slot = Slot::<Result<(), String>>::default();
        slot.run_async(&egui::Context::default(), rt.handle(), std::future::pending());
        drop(rt);
        assert!(slot.tick_result());
        assert!(matches!(slot.last, Some(Err(_))));
        assert!(!slot.busy());
    }
}
