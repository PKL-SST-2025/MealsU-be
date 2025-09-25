use axum::{
    extract::State,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::state::AppState;
use crate::routes::auth::validate_token; // Menggunakan validator token dari auth.rs

fn derive_name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("");
    let cleaned = local.replace(['.', '_', '-'], " ");
    cleaned
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserProfile {
    name: Option<String>,
    email: String,
    dietary_preference: Option<String>,
    gender: Option<String>,
    age: Option<i32>,
    bio: Option<String>,
    avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfilePayload {
    name: String,
    dietary_preference: String,
    gender: String,
    age: i32,
    bio: String,
}

pub fn users_router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_current_user).put(update_current_user))
        .route("/me/measurements", get(get_measurements).put(update_measurements))
}

async fn get_current_user(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    println!("🔍 get_current_user called");

    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");

    println!("🔑 Token received: {}", if token.is_empty() { "NONE" } else { "PRESENT" });

    let user_email = match validate_token(&state, token) {
        Ok(email) => {
            println!("✅ Token valid for user: {}", email);
            email
        },
        Err(_) => {
            println!("❌ Invalid token");
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        },
    };

    println!("🔍 Querying database for user: {}", user_email);
    match sqlx::query_as::<_, UserProfile>("SELECT email, name, dietary_preference, gender, age, bio, avatar FROM users WHERE email = $1")
        .bind(user_email)
        .fetch_one(&state.pool)
        .await
    {
        Ok(mut profile) => {
            if profile.name.as_deref().unwrap_or("").trim().is_empty() {
                let derived = derive_name_from_email(&profile.email);
                println!("ℹ️ Name empty, derived from email: {}", derived);
                profile.name = Some(derived);
            }
            println!("✅ Profile found: {:?}", profile);
            (StatusCode::OK, Json(profile)).into_response()
        },
        Err(e) => {
            println!("❌ Database error: {:?}", e);
            println!("❌ Error type: {}", std::any::type_name_of_val(&e));
            (StatusCode::NOT_FOUND, "User not found").into_response()
        },
    }
}

async fn update_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProfilePayload>,
) -> impl IntoResponse {
    println!("🔄 update_current_user called");
    println!("📦 Payload received: {:?}", payload);

    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");

    println!("🔑 Token received: {}", if token.is_empty() { "NONE" } else { "PRESENT" });

    let user_email = match validate_token(&state, token) {
        Ok(email) => {
            println!("✅ Token valid for user: {}", email);
            email
        },
        Err(_) => {
            println!("❌ Invalid token");
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        },
    };

    println!("🔍 Updating profile for user: {}", user_email);
    let result = sqlx::query(
        "UPDATE users SET name = $1, dietary_preference = $2, gender = $3, age = $4, bio = $5 WHERE email = $6"
    )
    .bind(payload.name)
    .bind(payload.dietary_preference)
    .bind(payload.gender)
    .bind(payload.age)
    .bind(payload.bio)
    .bind(user_email)
    .execute(&state.pool)
    .await;

    match result {
        Ok(res) => {
            println!("✅ Profile updated successfully, rows affected: {}", res.rows_affected());
            (StatusCode::OK, "Profile updated").into_response()
        },
        Err(e) => {
            println!("❌ Database error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update profile").into_response()
        },
    }
}

// --- Body Measurements --- //

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct UserMeasurements {
    height: Option<f32>,
    current_weight: Option<f32>,
    target_weight: Option<f32>,
    waist: Option<f32>,
    chest: Option<f32>,
    thigh: Option<f32>,
    arm: Option<f32>,
}

async fn get_measurements(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let user_email = match validate_token(&state, headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|s| s.strip_prefix("Bearer ")).unwrap_or("")) {
        Ok(email) => email,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    };

    let user_id_result = sqlx::query!("SELECT id FROM users WHERE email = $1", user_email).fetch_one(&state.pool).await;
    let user_id = match user_id_result {
        Ok(rec) => rec.id,
        Err(_) => return (StatusCode::NOT_FOUND, "User not found").into_response(),
    };

    match sqlx::query_as::<_, UserMeasurements>("SELECT height, current_weight, target_weight, waist, chest, thigh, arm FROM user_measurements WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
    {
        Ok(Some(measurements)) => (StatusCode::OK, Json(measurements)).into_response(),
        Ok(None) => (StatusCode::OK, Json(serde_json::json!({ // Return default values if no record exists
            "height": 0, "current_weight": 0, "target_weight": 0, "waist": 0, "chest": 0, "thigh": 0, "arm": 0
        }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch measurements").into_response(),
    }
}

async fn update_measurements(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UserMeasurements>,
) -> impl IntoResponse {
    let user_email = match validate_token(&state, headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|s| s.strip_prefix("Bearer ")).unwrap_or("")) {
        Ok(email) => email,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    };

    let user_id_result = sqlx::query!("SELECT id FROM users WHERE email = $1", user_email).fetch_one(&state.pool).await;
    let user_id = match user_id_result {
        Ok(rec) => rec.id,
        Err(_) => return (StatusCode::NOT_FOUND, "User not found").into_response(),
    };

    let result = sqlx::query!(
        r#"
        INSERT INTO user_measurements (user_id, height, current_weight, target_weight, waist, chest, thigh, arm, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        ON CONFLICT (user_id)
        DO UPDATE SET
            height = EXCLUDED.height,
            current_weight = EXCLUDED.current_weight,
            target_weight = EXCLUDED.target_weight,
            waist = EXCLUDED.waist,
            chest = EXCLUDED.chest,
            thigh = EXCLUDED.thigh,
            arm = EXCLUDED.arm,
            updated_at = NOW();
        "#,
        user_id,
        payload.height,
        payload.current_weight,
        payload.target_weight,
        payload.waist,
        payload.chest,
        payload.thigh,
        payload.arm
    )
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (StatusCode::OK, "Measurements updated").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update measurements").into_response(),
    }
}
