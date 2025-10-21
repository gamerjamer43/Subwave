use crate::{
    mods::models::{AuthRequest, Claims},
    api::{router::status_response}
};

// TODO: potentially migrate off sqlx. it's quite heavy
use sqlx::{
    PgPool, query,
    query_scalar
};

// axum high level server
use axum::{
    body::Body,
    response::Response,
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
};

// argon 2 hashing
use argon2::{
    password_hash::{
        PasswordHash, SaltString, 
        rand_core::{OsRng, RngCore}
    },
    Algorithm, Argon2, Version, Params, 
    PasswordHasher, PasswordVerifier
};

// jwt helpers
use chrono::Utc;
use jsonwebtoken::{
    decode, encode,
    DecodingKey, EncodingKey, 
    Header, Validation
};

// doing this w lazy evaluation so i only have to do this once. fuck you rust!!! you have brought me so much joy and so much pain
use std::sync::LazyLock;

// argon 2 password hasher
static ARGON2: std::sync::LazyLock<Argon2<'static>> = LazyLock::new(|| {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(8000, 2, 1, None)
            .expect("valid argon2 params"),
    )
});

// jwt secret key (32 byte token that changes each time you run)
// TODO: add set tokens so session strores arent useless
const JWT_SECRET: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret.to_vec()
});
const UPSERTSESSION: &str = include_str!("../queries/upsertsession.sql");

pub async fn signup(
    State(pool): State<PgPool>,
    Json(req): Json<AuthRequest>,
) -> Response<Body> {
    match async {
        // check if the user already exists
        if query("SELECT 1 FROM users WHERE username = $1")
            .bind(&req.username)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .is_some() {
                return Err(StatusCode::UNAUTHORIZED);
            }

        // salt n hash
        let password = ARGON2
            .hash_password(req.password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .to_string();

        // store
        query("INSERT INTO users (username, password) VALUES ($1, $2)")
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
    .await {
        Ok(resp) => resp,
        Err(status) => status_response(status),
    }
}

// still a big ass bottleneck in verification. we at 220 ms now with INSECURITY!!! magic numbers need to be removed too but this shit works so why not push
pub async fn login(
    State(pool): State<PgPool>,
    Json(req): Json<AuthRequest>,
) -> Response<Body> {
    match async {
        let hash: String = query_scalar("SELECT password FROM users WHERE username = $1")
            .bind(&req.username)
            .fetch_one(&pool).await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        ARGON2.verify_password(
            req.password.as_bytes(),
            &PasswordHash::new(&hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ).map_err(|_| StatusCode::UNAUTHORIZED)?;

        // create jwt
        let now = Utc::now().timestamp() as usize;
        let jwt = encode(
            &Header::default(),
            &Claims {
                // defaulted to 24 hours fuck w it as you wish
                sub: req.username.clone(),
                iat: now,
                exp: now + 86400,
            },
            &EncodingKey::from_secret(&JWT_SECRET),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // token will drop if i don't clone dat jawn
        let token_for_db = jwt.clone();
        let username = req.username.clone();
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            if let Err(e) = query(&UPSERTSESSION)
                .bind(&username)
                .bind(&token_for_db)
                .bind(now as i64)
                .execute(&pool_clone).await {
                    eprintln!("session insert failed: {e}");
                }
        });

        // build a response
        Ok(Response::builder()
            .header("Content-Type", "application/json")
            .body(Body::from(jwt))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
    }.await {
        Ok(resp) => resp,
        Err(status) => status_response(status),
    }
}

// TODO: check db for session so sessions aren't useless
pub async fn verify(
    pool: &PgPool,
    headers: &HeaderMap
) -> Result<(), StatusCode> {
    // decode header
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::trim))
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_string();

    // decode and verify jwt
    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(&JWT_SECRET),
        &Validation::default(),
    ).map_err(|e| {
        eprintln!("JWT decode failed: {:?}", e);
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    // jwt lib handles expiration check automatically (sends back a 422 if fucky)
    query("SELECT 1 FROM users WHERE username = $1")
        .bind(&token_data.claims.sub)
        .fetch_one(pool).await
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    // success
    Ok(())
}