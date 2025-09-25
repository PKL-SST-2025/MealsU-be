use axum::{routing::get, Router};

pub mod health;
use health::{health_handler, ping_handler};

pub mod auth;
use axum::routing::post;
use auth::{login, me, register, logout};

pub mod users;
use users::users_router;

use crate::state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_handler))
        .route("/ping", get(ping_handler))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/logout", post(logout))
        .nest("/users", users_router())
}
