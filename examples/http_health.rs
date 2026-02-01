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

    // Create router with health endpoint
    let app: Router<()> = Router::new().route(
        "/health",
        get({
            let logger = logger.clone();
            || async move { Json(logger.get_health_status()) }
        }),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Health check endpoint would listen on {addr}");
    println!("Router type: {:?}", std::any::type_name_of_val(&app));
    
    // Skip actual server startup for this example
    println!("Server would start here. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    println!("Received shutdown signal");

    Ok(())
}
