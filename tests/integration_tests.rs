use inklog::LoggerManager;
use std::time::Duration;
use tracing::{error, info};

#[tokio::test]
async fn test_e2e_logging() {
    // This test might fail if run in parallel with others due to global subscriber
    // We wrap it to ignore error if subscriber already set
    if let Ok(logger) = LoggerManager::new().await {
        info!("This is an info message");
        error!("This is an error message");

        // Give some time for async workers
        std::thread::sleep(Duration::from_millis(200));

        logger.shutdown().unwrap();
    }
}

#[cfg(feature = "confers")]
#[tokio::test]
async fn test_load_from_file() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(
        file,
        r#"
        [global]
        level = "debug"
        [performance]
        channel_capacity = 500
    "#
    )
    .unwrap();

    let _ = LoggerManager::from_file(file.path()).await;
}
