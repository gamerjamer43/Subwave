use serde::Serialize;

#[derive(Serialize)]
pub struct Song {
    pub id: u16,

    // song info
    pub name: String,
    pub artist: String,
    pub album: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<u8>>,

    // file info
    pub duration: u16,
    pub filename: String,
}


#[derive(serde::Serialize)]
pub struct Album {
    pub id: u16,
    
    // album info
    pub name: String,
    pub artist: String,
    pub runtime: u16,
    pub songcount: u8,

    // ref to each song
    pub songs: Vec<Song>,
}