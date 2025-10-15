use rand_core::{OsRng, RngCore};
use hyper::{Body, Request, Response, StatusCode};
use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use chrono::{Utc, Duration};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

use argon2::{password_hash::{PasswordHash, SaltString}, Argon2, Params, PasswordHasher, PasswordVerifier, Algorithm, Version};

#[derive(Deserialize)]
pub struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,  // username
    exp: usize,   // expiration time
    iat: usize,   // issued at
}

// doing this w lazy evaluation so i only have to do this once. fuck you rust!!! you have brought me so much joy and so much pain
static ARGON2: LazyLock<Argon2<'static>> = LazyLock::new(|| {
    let params = Params::new(4096, 2, 2, None).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
});

// jwt secret key - in production load from env var
static JWT_SECRET: LazyLock<Vec<u8>> = LazyLock::new(|| {
    std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| {
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            hex::encode(secret)
        })
        .into_bytes()
});

pub async fn signup(pool: SqlitePool, req: AuthRequest) -> Result<Response<Body>, StatusCode> {
    // check if the user already exists
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query("SELECT 1 FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // can't make 2 of the same account
    if !row.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // salt n hash
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = ARGON2.hash_password(req.password.as_bytes(), &salt)
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

// still a big ass bottleneck in verification. we at 220 ms now with INSECURITY!!! magic numbers need to be removed too but this shit works so why not push
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
    if ARGON2.verify_password(req.password.as_bytes(), &hash).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // generate jwt with 24 hour expiry
    let now = Utc::now();
    let claims = Claims {
        sub: req.username.clone(),
        exp: (now + Duration::hours(24)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&JWT_SECRET)
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // send that jawn back
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Body::from(token))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

pub async fn verify(pool: &SqlitePool, req: &Request<Body>) -> Result<(), StatusCode> {
    // decode header
    let auth = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // snag token from that shit
    let token = auth
        .strip_prefix("Bearer ")
        .map(str::trim)
        .ok_or(StatusCode::BAD_REQUEST)?;

    // decode and verify jwt
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&JWT_SECRET),
        &Validation::default()
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // jwt lib handles expiration check automatically, but we can verify user still exists if needed
    let _row = sqlx::query("SELECT 1 FROM users WHERE username = ?")
        .bind(&token_data.claims.sub)
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // success — caller can proceed
    Ok(())
}