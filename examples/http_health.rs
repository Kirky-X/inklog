use axum::{routing::get, Json, Router};
use inklog::LoggerManager;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let logger = Arc::new(LoggerManager::new().await.unwrap());

    let app = Router::new().route(
        "/health",
        get({
            let logger = logger.clone();
            move || async move {
                let status = logger.get_health_status();
                Json(status)
            }
        }),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Health check listening on {}", addr);
    let _listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
