-- song indexing
CREATE TABLE IF NOT EXISTS songs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,

    -- album info, we can get the cover and artist frm there
    album_id INTEGER NOT NULL,
    track_number INTEGER NOT NULL,

    duration INTEGER NOT NULL,
    filename TEXT NOT NULL UNIQUE,
    
    -- if we don't have a full albums worth of songs, order by where it is in the album
    FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    name TEXT NOT NULL,
    artist TEXT NOT NULL,
    cover BLOB,

    runtime INTEGER NOT NULL,
    songcount INTEGER NOT NULL
);

-- user info
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    password TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS playlists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    userId INTEGER NOT NULL,
    name TEXT NOT NULL,
    FOREIGN KEY (userId) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sessions (
    username TEXT PRIMARY KEY,
    token TEXT NOT NULL
);