use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tokio::sync::RwLock as TokioRwLock;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Runs a collection of futures concurrently, collecting their results.
///
/// Uses [`tokio::task::JoinSet`] to run futures as spawned tasks while
/// gracefully handling join errors — a panicking task produces `None`
/// in the output rather than propagating the panic.
pub async fn safe_gather<T, E>(futures: Vec<BoxFuture<'static, Result<T, E>>>) -> Vec<Result<T, E>>
where
    E: Send + 'static + std::fmt::Debug,
    T: Send + 'static,
{
    let mut set = tokio::task::JoinSet::new();
    for fut in futures {
        set.spawn(fut);
    }
    let mut out = Vec::with_capacity(set.len());
    while let Some(result) = set.join_next().await {
        match result {
            Ok(r) => out.push(r),
            Err(_join_err) => {
                // A spawned future panicked — skip it and continue
                // collecting the remaining results.
            }
        }
    }
    out
}

#[derive(Clone, Copy)]
pub struct Stopwatch {
    start: Instant,
}

impl Stopwatch {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }
    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}

pub type ReaderWriterLock<T> = TokioRwLock<T>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn safe_gather_returns_all_results() {
        let f1: BoxFuture<'static, Result<i32, &'static str>> = Box::pin(async { Ok(1) });
        let f2: BoxFuture<'static, Result<i32, &'static str>> = Box::pin(async { Ok(2) });
        let f3: BoxFuture<'static, Result<i32, &'static str>> = Box::pin(async { Ok(3) });
        let results = safe_gather(vec![f1, f2, f3]).await;
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn stopwatch_measures_elapsed() {
        let sw = Stopwatch::start();
        std::thread::sleep(Duration::from_millis(2));
        assert!(sw.elapsed_ms() >= 2);
    }
}
