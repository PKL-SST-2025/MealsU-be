use std::time::{SystemTime, Duration};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum::http::HeaderMap;
use jsonwebtoken::{encode, decode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use argon2::{PasswordHash, PasswordVerifier, PasswordHasher};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub async fn register(State(state): State<AppState>, Json(payload): Json<RegisterRequest>) -> impl IntoResponse {
    // basic validation
    if payload.email.trim().is_empty() || payload.password.len() < 6 {
        return (StatusCode::BAD_REQUEST, "Invalid email or password length").into_response();
    }

    // hash password
    let password_hash = match hash_password(&payload.password) {
        Ok(h) => h,
        Err((code, msg)) => return (code, msg).into_response(),
    };

    let user_id = Uuid::new_v4();

    let res = sqlx::query(
        "INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)"
    )
        .bind(user_id)
        .bind(&payload.email)
        .bind(&password_hash)
        .execute(&state.pool)
        .await;

    if let Err(e) = res {
        // unique violation (duplicate email)
        let msg = if let Some(db_err) = e.as_database_error() {
            if db_err.message().contains("unique") { "Email already registered" } else { "Failed to register" }
        } else { "Failed to register" };
        return (StatusCode::CONFLICT, msg).into_response();
    }

    let token = match issue_token(&state, &payload.email) {
        Ok(t) => t,
        Err((code, msg)) => return (code, msg).into_response(),
    };

    (StatusCode::CREATED, Json(AuthResponse { token })).into_response()
}

pub async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    let row = sqlx::query("SELECT password_hash FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);

    let Some(row) = (match row { Ok(r) => r, Err(code) => return code.into_response() }) else {
        return (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    };

    let password_hash: String = row.get("password_hash");
    let verified = match verify_password(&payload.password, &password_hash) {
        Ok(v) => v,
        Err((code, msg)) => return (code, msg).into_response(),
    };
    if !verified {
        return (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    }

    let token = match issue_token(&state, &payload.email) {
        Ok(t) => t,
        Err((code, msg)) => return (code, msg).into_response(),
    };
    (StatusCode::OK, Json(AuthResponse { token })).into_response()
}

#[derive(Serialize)]
pub struct MeResponse { pub email: String }

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");

    match validate_token(&state, token) {
        Ok(email) => (StatusCode::OK, Json(MeResponse { email })).into_response(),
        Err(_) => (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    }
}

fn hash_password(password: &str) -> Result<String, (StatusCode, &'static str)> {
    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
    let argon = argon2::Argon2::default();
    let password_hash = argon
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Hashing failed"))?
        .to_string();
    Ok(password_hash)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, (StatusCode, &'static str)> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid hash"))?;
    let argon = argon2::Argon2::default();
    Ok(argon.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

fn issue_token(state: &AppState, email: &str) -> Result<String, (StatusCode, &'static str)> {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or(Duration::from_secs(0));
    let exp = (now + Duration::from_secs(60 * 60 * 24 * 7)).as_secs() as usize; // 7 days
    let claims = Claims { sub: email.to_string(), exp };

    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(state.jwt_secret.as_bytes()))
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Token signing failed"))
}

pub fn validate_token(state: &AppState, token: &str) -> Result<String, ()> {
    let data = decode::<Claims>(token, &DecodingKey::from_secret(state.jwt_secret.as_bytes()), &Validation::new(Algorithm::HS256))
        .map_err(|_| ())?;
    Ok(data.claims.sub)
}

pub async fn logout() -> impl IntoResponse {
    (StatusCode::OK, "Logged out successfully")
}
