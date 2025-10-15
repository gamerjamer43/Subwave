use rand_core::{OsRng, RngCore};
use hyper::{Body, Response, StatusCode};
use sqlx::{SqlitePool, Row};
use serde::Deserialize;
use argon2::{password_hash::{PasswordHash, SaltString}, Argon2, Params, PasswordHasher, PasswordVerifier};

#[derive(Deserialize)]
#[allow(dead_code)]
// tagging this with a dead code tag cuz i'm not actually doing logins yet, or even like storing the token... just sending it
pub struct AuthRequest {
    username: String,
    password: String,
}

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
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(4096, 2, 2, None).unwrap() // just set p=2
    );
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
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,

        // turn this into a constant
        Params::new(4096, 1, 2, None).unwrap()
    );
    if argon2.verify_password(req.password.as_bytes(), &hash).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // generate a random 32 byte session token
    let mut token_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut token_bytes);
    let token = hex::encode(token_bytes);

    // salt n hash that too
    let salt = SaltString::generate(&mut OsRng);
    let token_hash = argon2.hash_password(token.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    // store in session table under the user
    sqlx::query("
        INSERT INTO sessions (username, token)
        VALUES (?, ?)
        ON CONFLICT(username) DO UPDATE SET token = excluded.token"
    )
      .bind(&req.username)
      .bind(&token_hash)
      .execute(&pool)
      .await
      .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // send that jawn back
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Body::from(token))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}