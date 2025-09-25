mod config;
mod routes;
mod db;
mod state;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::prelude::*;

use config::AppConfig;
use routes::api_router;
use state::AppState;

#[tokio::main]
async fn main() {
    init_tracing();

    let cfg = AppConfig::from_env();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Init database pool
    let pool = db::create_pool(&cfg.database_url)
        .await
        .expect("failed to connect to database");

    let state = AppState {
        pool,
        jwt_secret: std::sync::Arc::new(cfg.jwt_secret.clone()),
    };

    let app = Router::new()
        .nest("/api/v1", api_router())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = cfg.addr();
    info!("Starting server on http://{} (env PORT={})", addr, cfg.port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("server error");
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,tower_http=info,axum=info"))
        .expect("invalid RUST_LOG value");

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .init();
}
