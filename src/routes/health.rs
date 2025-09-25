use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
}

pub async fn health_handler() -> impl IntoResponse {
    Json(Health {
        status: "ok",
        service: "mealsu-be",
    })
}

#[derive(Serialize)]
struct Pong {
    message: &'static str,
}

pub async fn ping_handler() -> impl IntoResponse {
    Json(Pong { message: "pong" })
}
