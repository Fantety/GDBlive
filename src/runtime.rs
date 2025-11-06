use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};

/// RuntimeManager 管理 Tokio 异步运行时
pub struct RuntimeManager {
    runtime: Arc<Runtime>,
}

impl RuntimeManager {
    /// 创建新的 RuntimeManager，使用多线程运行时
    pub fn new() -> Self {
        let runtime = Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        Self {
            runtime: Arc::new(runtime),
        }
    }
    
    /// 创建使用当前线程的 RuntimeManager（用于测试）
    #[allow(dead_code)]
    pub fn new_current_thread() -> Self {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        Self {
            runtime: Arc::new(runtime),
        }
    }

    /// 返回运行时的克隆引用
    #[allow(dead_code)]
    pub fn runtime(&self) -> Arc<Runtime> {
        self.runtime.clone()
    }

    /// 在运行时上生成异步任务
    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        // spawn 返回 JoinHandle，但我们不需要等待它
        // Tokio 会自动管理任务的生命周期
        let _ = self.runtime.spawn(future);
    }

    /// 阻塞当前线程直到 future 完成
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}
