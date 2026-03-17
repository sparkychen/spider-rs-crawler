use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

/// 初始化结构化日志
pub fn init_logger() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .json()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    Ok(())
}
