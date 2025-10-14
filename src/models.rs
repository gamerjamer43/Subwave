use serde::Serialize;

#[derive(Serialize)]
pub struct Song {
    pub id: i64,

    // song info
    pub name: String,
    pub artist: String,
    pub album: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<u8>>,

    // file info
    pub duration: i16,
    pub filename: String,
}


#[derive(serde::Serialize)]
pub struct Album {
    pub id: i64,
    
    // album info
    pub name: String,
    pub artist: String,
    pub runtime: i64,
    pub songcount: i64,

    // ref to each song
    pub songs: Vec<Song>,
}