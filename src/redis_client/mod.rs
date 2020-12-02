use tokio::sync::{Mutex, MutexGuard};
use std::error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;


/// A Helper type just for th redis pool stuff
pub type RedisPool = Arc<AioRedisPool>;


/// Uses a set of rotating aio redis connections to balance load
/// and to also allow multiple things to be interacting with redis
/// due to the need of a mutex for each connection to keep interior
/// mutability because of the nature of the WS server with warp.
pub struct AioRedisPool {
    counter: AtomicUsize,
    clients: Vec<Mutex<redis::aio::Connection>>,
}

impl AioRedisPool {
    /// Spawns and connects to redis with n amount of clients which are
    /// then wrapped in a mutex and added to a internal queue.
    ///
    /// Can return either Self or a RedisError.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn error::Error>> {
    ///     let pool = RedisPool::create_clients("redis://127.0.0.1/", 5).await?;
    /// }
    /// ```
    pub async fn create_clients(host: &str, n: usize) -> Result<RedisPool, Box<dyn error::Error>>{
        let client = redis::Client::open(host)?;
        let mut clients = Vec::with_capacity(n);
        for _ in 0..n {
            let con = client.get_async_connection().await?;
            let wrapped_conn = Mutex::new(con);
            clients.push(wrapped_conn);
        }

        let slf = AioRedisPool {
            counter: AtomicUsize::new(0),
            clients,
        };
        Ok(Arc::new(slf))
    }

    /// Acquire a temporary reference to a async connection contained within
    /// the pool, this is a immutable operation due to the use of atomic
    /// operations for rotating the list index.
    ///
    /// This will panic if the index has somehow gone out of bounds of the list
    /// which should be impossible.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn error::Error>> {
    ///     let pool = RedisPool::create_clients("redis://127.0.0.1/", 5).await?;
    ///
    ///     let con = pool.acquire().await;
    /// }
    /// ```
    pub async fn acquire(&self) -> MutexGuard<'_, redis::aio::Connection> {
        let client = match self.clients.get(self.get_next()) {
            Some(c) => c,
            _ => panic!("Client was none on a valid index.")
        };

        let cli = client.lock().await;

        cli
    }

    fn get_next(&self) -> usize {
        let index = self.counter.fetch_add(1, Ordering::Relaxed);
        if index >= self.clients.len() - 1 {
            self.counter.store(0, Ordering::Relaxed);
        }

        index
    }
}