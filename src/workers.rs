use parking_lot::Mutex;
use threadpool;

lazy_static! {
    // Global worker thread pool provider.
    static ref THREAD_POOL_PROVIDER: ThreadPoolProvider = {
        info!("Building worker thread pool.");
        ThreadPoolProvider::new(threadpool::Builder::new().thread_name("drakonid-worker".into()).build())
    };
}

thread_local! {
    // Per-thread instance of ThreadPool
    static THREAD_POOL_INST: threadpool::ThreadPool = THREAD_POOL_PROVIDER.handle();
}

// Wrapper around ThreadPool to allow sharing initial references via Context
struct ThreadPoolProvider {
    pool: Mutex<threadpool::ThreadPool>,
}

impl ThreadPoolProvider {
    pub fn new(pool: threadpool::ThreadPool) -> Self {
        ThreadPoolProvider { pool: Mutex::new(pool) }
    }

    fn handle(&self) -> threadpool::ThreadPool {
        debug!("Generating new thread pool handle.");
        self.pool.lock().clone()
    }
}

#[inline]
pub fn run_on_worker<T: FnOnce(&threadpool::ThreadPool) -> ()>(thunk: T) {
    THREAD_POOL_INST.with(thunk);
}