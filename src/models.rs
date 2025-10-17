use serde::{Deserialize, Serialize};
use sqlx::{FromRow};

// song models (ig i32 is the move... resizing is slow)
#[derive(FromRow, Serialize)]
pub struct Song {
    pub id: i32,

    // song info
    pub name: String,
    pub artist: String,
    pub album: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<u8>>,

    // file info
    pub duration: i32,
    pub filename: String,
}

#[derive(serde::Serialize)]
pub struct Album {
    pub id: i32,
    
    // album info
    pub name: String,
    pub artist: String,
    pub runtime: i32,
    pub songcount: i32,

    // ref to each song
    pub songs: Vec<Song>,
}

// request models
#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize
}