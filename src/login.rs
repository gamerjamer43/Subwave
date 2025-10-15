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
    password: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize
}

// doing this w lazy evaluation so i only have to do this once. fuck you rust!!! you have brought me so much joy and so much pain
static ARGON2: LazyLock<Argon2<'static>> = LazyLock::new(|| {
    let params = Params::new(4096, 2, 2, None).expect("valid argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
});

// jwt secret key (gonna add an env for this but otherwise uses a random one)
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
    let row = sqlx::query("SELECT password, id FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // idk why this is 2 lines... but whatever. im too tired
    let hash_str: String = row.try_get("password")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hash = PasswordHash::new(&hash_str)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // verify password
    if ARGON2.verify_password(req.password.as_bytes(), &hash).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // cool jwt token
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: req.username.clone(),
        iat: now as usize,
        exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
    };

    // encode for sending
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&JWT_SECRET),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // store session
    sqlx::query("INSERT INTO sessions (username, token, issued) VALUES (?, ?, ?) 
                     ON CONFLICT(username) DO UPDATE SET token=excluded.token, issued=excluded.issued")
        .bind(&req.username)  // username
        .bind(&token)         // JWT token
        .bind(now)            // issued timestamp
        .execute(&pool)
        .await
        .map_err(|e| {
            eprintln!("Failed to insert session: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // slide er on over private
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
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
        token.trim(),
        &DecodingKey::from_secret(&JWT_SECRET),
        &Validation::default()
    ).map_err(|e| {
        eprintln!("JWT decode failed: {:?}", e);
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    // jwt lib handles expiration check automatically (sends back a 422 if fucky)
    let _row = sqlx::query("SELECT 1 FROM users WHERE username = ?")
        .bind(&token_data.claims.sub)
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    // success
    Ok(())
}