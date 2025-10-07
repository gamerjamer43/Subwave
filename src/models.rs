use serde::Serialize;

#[derive(Serialize)]
pub struct Song {
    pub id: i64,
    pub name: String,
    pub artist: String,
    pub album: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Vec<u8>>,
    pub duration: i16,
    pub filename: String,
}