use std::{env, net::SocketAddr};

pub struct AppConfig {
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        // Load .env if present
        let _ = dotenvy::dotenv();

        let port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080);

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/mealsu".to_string());

        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "dev_secret_change_me".to_string());

        Self { port, database_url, jwt_secret }
    }

    pub fn addr(&self) -> SocketAddr {
        ([0, 0, 0, 0], self.port).into()
    }
}
