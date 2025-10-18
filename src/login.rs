// backend
use hyper::{Body, Request, Response, StatusCode};
use sqlx::PgPool;

// Standard library
use std::sync::LazyLock;

// the tools ACTUALLY needed
use chrono::Utc;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use argon2::{password_hash::{PasswordHash, SaltString}, Argon2, Params, PasswordHasher, PasswordVerifier, Algorithm, Version};

// (also tokens expire on restart this way)
use rand_core::{OsRng, RngCore};

// request models
use crate::models::{AuthRequest, Claims};

// doing this w lazy evaluation so i only have to do this once. fuck you rust!!! you have brought me so much joy and so much pain
static ARGON2: LazyLock<Argon2<'static>> = LazyLock::new(|| {
    Argon2::new(Algorithm::Argon2id, Version::V0x13,
         Params::new(8000, 2, 1, None).expect("valid argon2 params"))
});

// jwt secret key (32 byte token that changes each time you run, will add set tokens soon)
static JWT_SECRET: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret.to_vec()
});

pub async fn signup(pool: PgPool, req: AuthRequest) -> Result<Response<Body>, StatusCode> {
    // check if the user already exists
    if sqlx::query("SELECT 1 FROM users WHERE username = $1")
        .bind(&req.username)
        .fetch_optional(&pool)
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some() {
            return Err(StatusCode::UNAUTHORIZED);
        }

    // salt n hash
    let password = ARGON2
        .hash_password(req.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    // store
    sqlx::query("INSERT INTO users (username, password) VALUES ($1, $2)")
        .bind(&req.username)
        .bind(&password)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .body(Body::from("User created"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

// still a big ass bottleneck in verification. we at 220 ms now with INSECURITY!!! magic numbers need to be removed too but this shit works so why not push
pub async fn login(pool: PgPool, req: AuthRequest) -> Result<Response<Body>, StatusCode> {
    // fetch hash and verify in one go
    let hash_str: String = sqlx::query_scalar("SELECT password FROM users WHERE username = $1")
        .bind(&req.username)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    ARGON2
        .verify_password(req.password.as_bytes(), &PasswordHash::new(&hash_str).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // create jwt
    let now = Utc::now().timestamp() as usize;
    let jwt = encode(
        &Header::default(),
        &Claims {
            // defaulted to 24 hours fuck w it as you wish
            sub: req.username.clone(), iat: now, exp: now + 86400 
        },
        &EncodingKey::from_secret(&JWT_SECRET),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // token will drop if i don't clone dat jawn
    let t = jwt.clone();
    tokio::spawn(async move {
        async move {
            if let Err(e) = sqlx::query("INSERT INTO sessions (username, token, issued) VALUES ($1, $2, $3) \
                                        ON CONFLICT (username) DO UPDATE SET token = EXCLUDED.token, issued = EXCLUDED.issued")
                .bind(&req.username)
                .bind(&t)
                .bind(now as i64)
                .execute(&pool)
                .await
            {
                eprintln!("session insert failed: {e}");
            }
        }
    });

    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(Body::from(jwt))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

pub async fn verify(pool: &PgPool, req: &Request<Body>) -> Result<(), StatusCode> {
    // decode header
    let token = req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::trim))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // decode and verify jwt
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&JWT_SECRET),
        &Validation::default()
    ).map_err(|e| { eprintln!("JWT decode failed: {:?}", e); StatusCode::UNPROCESSABLE_ENTITY })?;

    // jwt lib handles expiration check automatically (sends back a 422 if fucky)
    sqlx::query("SELECT 1 FROM users WHERE username = $1")
        .bind(&token_data.claims.sub)
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    // success
    Ok(())
}