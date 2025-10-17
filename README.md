<h1 align="center"><img src="https://github.com/user-attachments/assets/9acd80d4-f555-4367-b54b-01b42bb575a7" width="35"> Subwave </h1><br>

<p align="center">
    No bullshit here, only raw speed. Streams fast, searches faster, and doesn’t get in your way.
</p>
<p align="center">
    <em>This project goes hand in hand with <a href="https://github.com/gamerjamer43/Hertzonic">Hertzonic</a>. Check it out!</em>
</p>

<p align="center">
    <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-blue.svg" />
    <img alt="Rust" src="https://img.shields.io/badge/rust-1.70+-grey.svg" />
    <img alt="Postgres" src="https://img.shields.io/badge/db-postgres-316192.svg" />
    <img alt="License" src="https://img.shields.io/badge/license-private-red.svg" />
</p>

## Table of Contents

- [Introduction](#introduction)
- [Features](#features)
- [Quickstart (dev)](#quickstart-dev)
- [Configuration](#configuration)
- [API Endpoints](#api-endpoints)
- [Database schema](#database-schema)
- [Development notes](#development-notes)
- [Roadmap](#roadmap)
- [Contributing](#contributing)

## Introduction

Subwave doesn’t mess around. It’s built for speed: every request, every stream, every search is optimized to move fast. Featuring lightning-quick HTTP handling and no-clog asynchronous DBing, it slices through tasks with almost zero overhead. Metadata is pre-indexed, files are served directly from `./static/`, and the JSON API keeps things lightweight — no unnecessary layers, no slowdowns.  
In short: Subwave is fast as hell because it does exactly what it needs to do, and nothing more.

## Features

- Built on **Hyper** for a smoking fast HTTP backend, and **sqlx** to carry the DBing on it's back. 
- Literally every single operation is optimized. Signups take about 50ms, starting a music stream takes less than a millisecond, **EVERYTHING** is optimized as fast (albeit not lacking in security) as it can be.
- Deals with all the login auth bullshit for you! A header is provided on login, pass it with your requests to be allowed to access shit. Session tokens expire on server restart or after 24 hours, so don't hardcode it.
- Super duper light executable. Only uses around 15 libraries which have a total of 272 dependencies, which I'm still working to chop down.

## Quickstart (dev)

All you need is Rust and [Postgres](https://www.postgresql.org/).

1) Clone the repo

```bash
git clone <repo-url>
cd flacend
```

2) Set an env variable (example PowerShell and bash):

```powershell
# ps
$env:DATABASE_URL = "postgres://postgres:postgres@localhost:5432/flacend"
```

```bash
# bash
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/flacend"
```

3) Build & run. The server makes the DB for you!

```bash
# host defaults to port 6000.
cargo build
cargo run
```

## Configuration
Quite possibly the most simple thing you will ever do. Just point the database URL:
- `DATABASE_URL` — Postgres connection string. Defaults to `postgres://postgres:postgres@localhost:5432/flacend` if not set. 

## API Endpoints

- Ungated:
    - `POST /api/signup` — Body JSON: `{ "username": "alice", "password": "secret" }`, 201 Created on success.
    - `POST/api/login` — Body JSON: `{ "username": "alice", "password": "secret" }`, 200 OK with token in response body (text)
        - Pass an auth header: `Authorization: Bearer <token>`, for any requests that require authorization.

- Auth. Required:
    - `GET /api/search?q=term` — JSON array of songs
    - `GET /api/cover/:id` — album art (image/jpeg)
    - `GET /api/album/:id` — album + song list
    - `GET /file/<path>` — serves the files in `./static/` by name.

## Database schema

I tried to design this super fuckin simply, the primary schema file lives at `queries/createdb.sql`. Legit just:
- `albums:` (id SERIAL, name, artist, cover bytea, runtime, songcount)
- `songs:` (id SERIAL, name, album_id, track_number, duration, filename)
- `users:` (id SERIAL, username UNIQUE, password)
- `playlists:` (unfinished... check back soon)

Note: The project used SQLite originally; schema and queries are adjusted for Postgres.

## Development notes

- `scanner::scan` indexes files under `./static/` and inserts metadata into the DB; if you fuck around with the server make sure it gets ran, either on startup or via a maintenance task.
- Sessions are the one thing that isn't persistent. It uses an in-memory `SessionStore` (regenning on startup) with a 24 hour TTL. If you want session persistence, jack the values up and set an environment variable for the JWT key. The way I have it is just decently secure.
- If you fuck around with anything, adding proper migrations (`sqlx-cli` or similar) might help. The `createdb.sql` file is cool and all but I needed to mess around a lot.

## Roadmap

The goal of this was to provide a competitor to existing Subsonic forks. I've heard a lot of complaints about login auth and speed on thing like Navidrome or Airsonic, and a lot of the other options don't fully emulate the Subsonic API. So the long goal is to create a **completely new** speed optimized and security hardened ecosystem that if others like, they can fuck around with.

### Immediate
- [ ] Song upload route
- [ ] Full playlist support (favorites goes hand in hand with this).
- [ ] Standard search and indexing improvements. Search right now is very basic.

### High Priority
- [ ] API rate limiting / abuse protection. Zero limiting as of right now, but that's b/c I've been working on other security stuff.
- [ ] Usage analytics (playcounts, favorites, and other shit).
- [ ] Collaborative playlists (share with a link, giving another user the ability to edit the playlist).

### Core Features
- [ ] Artist pages endpoints (artist metadata, album lists).
- [ ] Sharing links (generate server-side share tokens / embed endpoints).
- [ ] Recommendations engine (simple similar-track heuristics).
- [ ] Admin & content moderation tooling

### Enhanced Experience
- [ ] Radio mode (auto suggest/play similar tracks)
- [ ] Lyrics + timing. (.vtt subtitle or .lrc synced lyric files)
- [ ] Recently played
- [ ] Optional transcoder pipeline (format conversion)
- [ ] Caching strategy / CDN-friendly headers

### Maybes
- [ ] Websocket or SSE hooks for the slower shit (nothing has caused any unfixable issues, but we'll see)
- [ ] ReplayGain normalization metadata support (store/read)

## Contributing

PRs are always welcome, but keep your changes small and focused on one specific element of the code (do as I say not as I do). Try to keep my coding style and if adding DB schema changes, include a migration.

## Acknowledgments

### Built with ❤️ using:
- [Hyper](https://hyper.rs/) - Quite possibly my favorite HTTP client for Rust. 
- [sqlx](https://github.com/launchbadge/sqlx) - No contest. The BEST SQL driver. Migrating to Postgres was super easy.
- [Argon2](https://github.com/sru-systems/rust-argon2) - Password hashing done right. Literally industry standard.

---

<p align="center">
    Made with 🎵 by <a href="https://github.com/gamerjamer43">gamerjamer43</a>
</p>

<p align="center">
    <sub>Stream responsibly. License your songs, and make sure to support your favorite artists.</sub>
</p>