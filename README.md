<h1 align="center"> Subwave 🌊 </h1>

<p align="center"><img src="https://github.com/user-attachments/assets/978945d3-da0f-4224-ae93-c7c6cf880fce" width="250"></p>
<p align="center">
    <img alt="Version" src="https://img.shields.io/badge/version-0.0.1%20indev-blue.svg" />
    <img alt="Rust" src="https://img.shields.io/badge/rust-1.70+-grey.svg" />
    <img alt="Postgres" src="https://img.shields.io/badge/db-postgres-316192.svg" />
    <img alt="License" src="https://img.shields.io/badge/license-private-red.svg" />
</p>
<p align="center"><b>Raw speed, for wherever you might be. Raw compatibility, on whatever you might use. One command, two install, and you're good. It's the new wave.</b></p>
<p align="center"><em>This project goes hand in hand with <a href="https://github.com/gamerjamer43/Hertzonic">Hertzonic</a>. Check it out!</em></p>

## Table of Contents

- [Introduction](#introduction)
- [Features](#features)
- [Quickstart (dev)](#quickstart-dev)
- [Configuration](#configuration)
- [API Endpoints](#api-endpoints)
- [Database schema](#database-schema)
- [Development logs](#development-logs)
- [Roadmap](#roadmap)
- [Documentation](#documentation)
- [Contributing](#contributing)

## Introduction

<h3 align="center">Subwave is a passion project meant to build on some of the painpoints of some of the existing options. Many of the most used self-hosting options are hair-yankingly painful to setup, so Subwave makes it simple, no extra setup beyond creating a single table. Designed to handle large amounts of music, so slowdowns will never be a concern — just plug and play wherever, whenever.</h3>

## Features

- Built on top of [Hyper](https://github.com/hyperium/hyper) using [Tower](https://github.com/tower-rs/tower) and [Axum](https://github.com/tokio-rs/axum) for a smoking fast HTTP backend, and [sqlx](https://github.com/launchbadge/sqlx) as an absolute unit to drive [Postgres](https://postgresql.org) on it's back.
- Literally every single operation is optimized. Signups take about 50ms, starting a music stream takes less than a millisecond, **EVERYTHING** is fine-tuned to be as fast (albeit still security tough and feature robust) as it can be.
- Deals with all the login auth bullshit for you! A header is provided on login, pass it with your requests to be allowed to access shit. Session tokens expire on server restart or after 24 hours, so don't hardcode it.
- Pen-tested, stress-tested, and idiot-tested. Whether it's 20 songs or 200 thousand, you will have zero problems with hosting this publicly, whether it be from a lack of attention or a malicious outside source.
- Air light executable. Only uses 14 uniquely important libs, which have a total of 287 dependencies.

## Quickstart (dev)

You only need two things to start the setup process, [Rust](https://rust-lang.org/) and [Postgres](https://www.postgresql.org/download/).

1) Clone the repo

```bash
git clone <repo-url>
cd Subwave
```

2) Set an environment variable (example PowerShell and bash):

```powershell
# ps
$env:DATABASE_URL = "postgres://user:password@localhost:5432/subwave"
```

```bash
# cmd
setx DATABASE_URL "postgres://user:password@localhost:5432/subwave"
```

```bash
# bash
export DATABASE_URL="postgres://user:password@localhost:5432/subwave"

```

3) Populate your DB with the schema this code needs. (defaulted to subwave):
```bash
psql -U username -d subwave -f queries/createdb.sql
```

4) Prep, build & run. The server autofills the DB for you!

```bash
# prep the db before hand for the compile time macros
cargo sqlx prepare

# host defaults to port 6000. use release for an optimized binary
cargo build # --release
cargo run
```

5) If you ever need a full clean:
```bash
# remove .sqlx/, then run
cargo clean
```

## Configuration
Quite possibly the most simple thing you will ever do. Just point the database URL (this is in the above steps):
- `DATABASE_URL` — Postgres connection string. This is the only thing besides the table you have to add!

## API Endpoints

- Ungated:
    - `POST /api/signup` — Response body: `{ "username": "alice", "password": "secret" }`, 201 Created on success.
    - `POST/api/login` — Response body: `{ "username": "alice", "password": "secret" }`, 200 OK with token in response body (text)
        - Pass a standard auth header `Authorization: Bearer <token>`, for any gated requests.

- Auth. Required:
    - `GET /api/search?q=term` — a basic select search from postgres using sql.
    - `GET /api/cover/:id` — album art (image/jpeg).
    - `GET /api/album/:id` — album and song list.
    - `GET /file/<path>` — serves the files in `./static/` by name.

## Database schema

I tried to design this super fuckin simply, the primary schema file lives at `queries/createdb.sql`. Legit just:
- `albums:` (id SERIAL, name, artist, cover bytea, runtime, songcount)
- `songs:` (id SERIAL, name, album_id, track_number, duration, filename)
- `users:` (id SERIAL, username UNIQUE, password)
- `playlists:` (unfinished... check back soon)

Note: The project used SQLite originally; schema and queries are adjusted for Postgres.

## Development logs

#### Known offenses:
- I need to do some more package optimization, but I'm modeling right now. I am sure I can chop down from 394 dependencies. The entire cargo build environment is 1.5GB right now...
- Stress testing is after I get covers out of the DB. This should be done by 0.0.9.
- 

#### Notes:
- `scanner::scan` indexes files under `./static/` and inserts metadata into the DB; if you fuck around with the server make sure it gets ran, either on startup or via a maintenance task.
- Sessions are the one thing that isn't persistent. It uses an in-memory `SessionStore` (regenning on startup) with a 24 hour TTL. If you want session persistence, jack the values up and set an environment variable for the JWT key. The way I have it is just decently secure.
- If you fuck around with anything, adding proper migrations (`sqlx-cli` or similar) might help. The `createdb.sql` file is cool and all but I needed to mess around a lot.
- There is also a metadata helper! Run metadata.py to add image and text metadata to a song if it doesn't have the data you need.

## Roadmap

This is meant to be a competitor to existing Subsonic forks. I've heard a lot of complaints about login auth, media DB corruption, sloggy performance on large song counts, and other unwanted garbage on platforms like Navidrome or Airsonic, and many of the other options don't fully achieve their goal of emulating the Subsonic API. This will, when done, be a **completely new,** speed optimized, and security hardened media server that if others like, they can fuck around with. Hey, you may even want to write your own media client when all is set and done.

### Immediate
- [ ] Add serialized errors instead of just bare status codes.
- [ ] Song upload route.
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
- [ ] On the fly transcoder (format conversion/compression)
- [ ] Caching strategy / CDN-friendly headers

### Maybes
- [ ] Websocket or SSE hooks for the slower shit (nothing has caused any unfixable issues, but we'll see)
- [ ] ReplayGain normalization metadata support (store/read)

## Documentation

I have not started on this, because as of right now there's only ~500 significant lines of rust driving this whole thing. Come back later for more!

## Contributing

PRs are always welcome, but keep your changes small and focused on one specific element of the code (do as I say not as I do). Try to keep my coding style and if adding DB schema changes, include a migration.

## Acknowledgments

### Built with ❤️ using:
- [Axum](https://github.com/tokio-rs/axum) - Quite possibly my favorite HTTP client for Rust, Hyper, has a way higher level version, for way more control. I like Axum a lot.
- [sqlx](https://github.com/launchbadge/sqlx) - No contest. The BEST SQL driver. Migrating to Postgres was super easy.
- [Argon2](https://github.com/sru-systems/rust-argon2) - Password hashing done right. Literally industry standard.
- [Postgres](https://postgresql.org) - You already know, it's industry standard. If you're hosting for multiple people, or hosting a shitload of songs, this DB will handle it, no problem.

---

<p align="center">
    Made with 🎵 by <a href="https://github.com/gamerjamer43">gamerjamer43</a>
</p>

<p align="center">
    <sub>Stream responsibly. License your songs, and make sure to support your favorite artists.</sub>
</p>