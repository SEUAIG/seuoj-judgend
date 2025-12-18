use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub(crate) fn init_logger(log_dir: impl AsRef<Path>) -> WorkerGuard {
    // 1. 定义日志文件路径和滚动策略（按天滚动，前缀为 judger.log）
    // 路径为 "./logs"，文件名会自动变为 judger.log.2023-10-27
    let file_appender = rolling::daily(log_dir, "judger.log");

    // 2. 创建一个非阻塞的写入器，防止写日志阻塞主逻辑（尤其是判题这种对时间敏感的任务）
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 3. 配置日志级别过滤器（优先从环境变量 RUST_LOG 读取，默认使用 info）
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 4. 组装订阅者并初始化
    tracing_subscriber::registry()
        .with(env_filter)
        // 输出到终端的层（美化打印）
        .with(fmt::layer().with_writer(std::io::stdout).pretty())
        // 输出到文件的层（JSON格式或其他，方便判题机后期分析）
        .with(
            fmt::layer()
                .with_ansi(false) // 文件中不需要彩色字符
                .with_writer(non_blocking)
        )
        .init();
    guard
}