# MealsU-be (Axum)

Backend dasar untuk MealsU menggunakan Rust + Axum.

## Fitur Awal
- Endpoint `GET /api/v1/health` -> `{ "status": "ok", "service": "mealsu-be" }`
- Endpoint `GET /api/v1/ping` -> `{ "message": "pong" }`
- CORS (allow all) untuk memudahkan pengembangan lokal
- Logging via tracing + tower-http trace
- Konfigurasi PORT via env (`.env` atau environment var)

## Menjalankan
1. Salin `.env.example` menjadi `.env` (opsional, default `PORT=8080`)
2. Build dan jalankan:

```bash
cargo run
```

Server akan berjalan di `http://127.0.0.1:8080` secara default.

## Struktur Proyek
```
MealsU-be/
├─ Cargo.toml
├─ .gitignore
├─ .env.example
├─ README.md
└─ src/
   ├─ main.rs
   ├─ config.rs
   └─ routes/
      ├─ mod.rs
      └─ health.rs
```

## Catatan
- Ubah kebijakan CORS saat production.
- Atur `RUST_LOG` untuk kontrol level log (contoh: `RUST_LOG=debug`).
