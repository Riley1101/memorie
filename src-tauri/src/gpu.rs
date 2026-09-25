//! Keeps model work from overlapping on the GPU.
//!
//! On Apple Silicon every model (the local LLM, the embedder, the reranker)
//! runs on one shared Metal device, and candle 0.8's Metal backend aborts the
//! whole process when two threads encode work on it at once. kalosm gives
//! every model that same device and offers no way to pass a separate one, so
//! the app runs one model job at a time instead. Other platforms run models on
//! the CPU, where concurrent use is safe, and never wait here.

use std::future::Future;
use tokio::sync::{Mutex, MutexGuard};

/// Whether models run on a device that can't take work from two threads at once.
const SERIALIZE: bool = cfg!(all(target_os = "macos", target_arch = "aarch64"));

static GPU: Mutex<()> = Mutex::const_new(());

tokio::task_local! {
    /// Set while a task runs inside `exclusive`, so model calls it makes don't
    /// wait on the lock it already holds.
    static HELD: ();
}

/// Waits for the GPU and holds it until the guard drops. `None` when there's
/// nothing to wait for: on CPU builds, or inside `exclusive`.
pub async fn lock() -> Option<MutexGuard<'static, ()>> {
    if !SERIALIZE || HELD.try_with(|_| ()).is_ok() {
        return None;
    }
    Some(GPU.lock().await)
}

/// Runs `job` holding the GPU throughout, for work that uses a model several
/// times in a row (a chat reply routes, searches, then generates) and shouldn't
/// let other model work in between.
pub async fn exclusive<F: Future>(job: F) -> F::Output {
    let _guard = lock().await;
    HELD.scope((), job).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn calls_inside_exclusive_do_not_wait_on_themselves() {
        let done = tokio::time::timeout(std::time::Duration::from_secs(2), exclusive(async {
            let inner = lock().await;
            assert!(inner.is_none());
            exclusive(async { 7 }).await
        }))
        .await;
        assert_eq!(done.unwrap(), 7);
    }

    #[tokio::test]
    async fn separate_jobs_take_turns() {
        if !SERIALIZE {
            return;
        }
        let first = lock().await;
        assert!(first.is_some());
        let second = tokio::time::timeout(std::time::Duration::from_millis(100), lock()).await;
        assert!(second.is_err(), "a second job got the GPU while the first held it");
        drop(first);
        assert!(lock().await.is_some());
    }
}
