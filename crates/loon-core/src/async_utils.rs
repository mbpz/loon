use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tokio::sync::RwLock as TokioRwLock;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub async fn safe_gather<T, E>(futures: Vec<BoxFuture<'static, Result<T, E>>>) -> Vec<Result<T, E>>
where
    E: Send + 'static + std::fmt::Debug,
    T: Send + 'static,
{
    let handles: Vec<_> = futures.into_iter().map(tokio::spawn).collect();
    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        match h.await {
            Ok(r) => out.push(r),
            Err(_join_err) => {
                panic!("safe_gather: task join error (cannot recover into generic E)");
            }
        }
    }
    out
}

pub struct Stopwatch {
    start: Instant,
}

impl Stopwatch {
    pub fn start() -> Self {
        Self { start: Instant::now() }
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
