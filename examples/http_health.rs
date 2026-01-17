// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! HTTP Health Check Endpoint Example
//!
//! This example demonstrates how to expose Inklog's health metrics
//! via an HTTP endpoint using Axum.

use axum::{routing::get, Json, Router};
use inklog::LoggerManager;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Arc::new(LoggerManager::new().await?);

    let app = Router::new().route(
        "/health",
        get({
            let logger = logger.clone();
            || async move { Json(logger.get_health_status()) }
        }),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Health check endpoint listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::Server::bind(&listener.local_addr()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
