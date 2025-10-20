CREATE TABLE IF NOT EXISTS albums (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    artist TEXT NOT NULL,
    cover BYTEA,
    runtime INTEGER NOT NULL,
    songcount INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS songs (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    album_id INTEGER NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    track_number INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    filename TEXT NOT NULL UNIQUE,

    UNIQUE (album_id, track_number)
);

CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS playlists (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    username TEXT PRIMARY KEY,
    token TEXT NOT NULL,
    issued INTEGER NOT NULL
);