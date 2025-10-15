use rand_core::{OsRng, RngCore};
use hyper::{Body, Response, StatusCode};
use sqlx::{SqlitePool, Row};
use serde::Deserialize;
use argon2::{Argon2, PasswordHasher, PasswordVerifier, password_hash::{SaltString, PasswordHash}};

#[derive(Deserialize)]
#[allow(dead_code)]
// tagging this with a dead code tag cuz i'm not actually doing logins yet, or even like storing the token... just sending it
pub struct AuthRequest {
    username: String,
    password: String,
}

pub async fn signup(pool: SqlitePool, req: AuthRequest) -> Result<Response<Body>, StatusCode> {
    // salt n hash
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(req.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    // store
    sqlx::query("INSERT INTO users (username, password) VALUES (?, ?)")
        .bind(&req.username)
        .bind(&password_hash)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .body(Body::from("User created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

pub async fn login(pool: SqlitePool, req: AuthRequest) -> Result<Response<Body>, StatusCode> {
    // fetch stored password hash
    let row = sqlx::query("SELECT password FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // properly parse hash
    let hash_str: String = row
        .try_get("password")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let hash = PasswordHash::new(&hash_str)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // verify against the hash
    let argon2 = Argon2::default();
    if argon2.verify_password(req.password.as_bytes(), &hash).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // generate a random 32 byte session token
    let mut token_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut token_bytes);
    let token = hex::encode(token_bytes);

    // send that jawn back
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Body::from(token))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}