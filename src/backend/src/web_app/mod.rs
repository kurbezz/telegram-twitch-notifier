pub mod auth;
pub mod subscriptions;
pub mod validation;

use std::net::SocketAddr;

use axum::Router;
use subscriptions::get_api_router;
use tokio::net::TcpListener;
use tower_http::services::ServeFile;

use crate::config::CONFIG;

fn get_app() -> Router {
    Router::new()
        .nest_service("/assets", ServeFile::new("assets"))
        .nest("/api", get_api_router())
        .fallback_service(ServeFile::new("assets/index.html"))
}

pub async fn start_web_app() -> Result<(), eyre::Report> {
    let app = get_app();

    let address = SocketAddr::new([0, 0, 0, 0].into(), CONFIG.telegram_mini_app_port);

    let _ = axum::serve(
        TcpListener::bind(address).await.unwrap(),
        app.into_make_service(),
    )
    .await;

    Ok(())
}
