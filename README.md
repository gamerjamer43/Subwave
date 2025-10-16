<h1 align="center"><img alt="Flashsound" title="Flashsound" src="https://github.com/user-attachments/assets/9acd80d4-f555-4367-b54b-01b42bb575a7" width="35"> Flashsound </h1><br>

<p align="center">
    A blazingly fast, cross-platform music streaming player built with a React for the web and Expo for mobile. Stream your music library from anywhere with beautiful metadata display, real-time search, and intuitive playback controls.
</p>

<p align="center">
    <em>This project goes hand in hand with <a href="https://github.com/gamerjamer43/Subwave">Subwave</a>. Check it out!</em>
</p>

<p align="center">
    <img alt="Version" src="https://img.shields.io/badge/version-1.0.0-blue.svg" />
    <img alt="React" src="https://img.shields.io/badge/react-19.2.0-61dafb.svg" />
    <img alt="TypeScript" src="https://img.shields.io/badge/typescript-5.9.2-3178c6.svg" />
    <img alt="Expo" src="https://img.shields.io/badge/expo-~54.0.12-000020.svg" />
    <img alt="License" src="https://img.shields.io/badge/license-private-red.svg" />
</p>

## Table of Contents

- [Introduction](#introduction)
- [Features](#features)
- [Tech Stack](#tech-stack)
- [Build Process](#build-process)
- [Build Layout](#build-layout)
- [Roadmap](#roadmap)
- [Contribution](#contribution)
- [Acknowledgments](#acknowledgments)

## Introduction

**Flashsound** is a modern, lightweight music streaming application that brings your entire music library to your fingertips. Built with React Native and Expo, it delivers native performance across iOS, Android, and Web platforms with a single codebase.

Whether you're streaming FLAC files, OGG Vorbis, or MP3s, Flashsound automatically extracts and displays rich metadata including album art, artist information, and track details. The real-time search functionality lets you find your favorite tracks instantly, while the intuitive playback controls give you complete command over your listening experience.

This is my first crack at learning how to use react, as it seems way more fun to use than normal HTML/CSS/JS. If I may have done anything weird with my code, please let me know at noah@noahmingolel.li. I'm super new to this shit and trying to do a seperate mobile and application interface, but I am not sure at all if that's the correct way to do it. There is also currently a decent amount of code debt and overengineering going on here. Once this is fully refactored, v0.1 will release.

Flashsound is **perfect** for audiophiles who demand quality without compromise. Take a look at [Subwave](https://github.com/gamerjamer43/Subwave) if you want a standardized backend, otherwise use [my instance](https://flacend.noahmingolel.li/) of it, which it's set to by default. This instance is invite only and will require approval before signing up.

<p align="center">
    <img src="https://img.shields.io/badge/Platform-iOS%20%7C%20Android%20%7C%20Web-success" />
</p>

## Features

A few of the things you can do with Flashsound:

- **Lossless-first streaming (FLAC-ready)** — Stream high-resolution FLAC, WAV, OGG, or any other lossless algo, compressed or uncompressed, from a self hosted server with adaptive buffering for super clean playback. Zero chop, zero skip delay, just pure performance.
- **Progressive background caching** — A LOT of caching to say the least. Album covers, songs, and data are all cached. No more 200ms delay when you go to load a song!
- **Dev-friendly debugging** — Error boundaries surface friendly recovery UI and debug logs, and the player automatically attempts reconnects and stream rebuffering on flaky networks.

These features keep Flashsound fast, responsive, and focused on quality, for cases as small as local tests, and as large as full production servers.

<p align="center">
    <i>You can have 2 of 3. Quality, fast, and cheap. I did all 3.</i>
</p>

## Tech Stack
This project is intentionally cross-platform: a single TypeScript codebase targets web (React + React DOM via React Native Web) and native (React Native on Expo). Below are the core technologies and their roles.

**Frontend (app)**:
- React 19.x — UI primitives and concurrent features used across web and native.
- React Native 0.81.x — Native components and platform APIs for iOS/Android builds.
- React Native Web — Bridges React Native components to the browser where appropriate (used selectively for layout and controls).
- Expo (~54) — App lifecycle, build tooling, and native module management for mobile targets.
- TypeScript 5.9 — Strict typing across the codebase (see interfaces in `src/interfaces/`).

**Audio, metadata & streaming:**
- HTML5 Audio API — Primary playback engine on web (lightweight, reliable).
- React Native audio (Expo AV / react-native-track-player, optional integrations) — Mobile playback with background mode support.
- Custom buffering layer that deals with adaptive buffering, reconnection, and CORS fallbacks, we don't fuck with dropped connection around here.

**UI / UX / styling:**
- Lucide React — Almost all the clip art icons I used were from here. These are super super sleek.
- Standard CSS and Native stylesheets.

**Developer & build tooling:**
- Node.js 18+ and npm/yarn — package management and scripts.
- ESLint + Prettier — linting and formatting (project convention).
- Metro bundler (React Native) and Vite/webpack for any web-specific tooling depending on Expo web build.

**Architecture notes:**
- Local persistence uses IndexedDB (web) and AsyncStorage (mobile) for queue and settings.
- The codebase favors small abstractions to make porting between web and native straightforward.

## Build Process

**Prerequisites:**
- Node.js 18+ installed
- Expo CLI (`npm install -g expo-cli`)
- iOS Simulator (for iOS development) or Android Studio (for Android development)
- A running music server on `http://127.0.0.1:5000` (for local development)

**Installation:**

```bash
# clone the repo
git clone https://github.com/gamerjamer43/Flashsound.git
cd Flashsound

# install dependencies
npm install

# start dev server
npx expo start
```

**Platform-Specific Builds:**

```bash
npm run ios
npm run android
npm run web
```

**Production Build:**

```bash
# build for production
expo build:android
expo build:ios
expo build:web
```

## Build Layout
```
src/
├── App.tsx                 # entry, routing and error boundaries
├── components/             # small, reusable components (controls, cards, sidebar, queue, search)
├── pages/                  # route-level screens (homepage, library, account, login)
├── player/                 # player container and helpers (`player.tsx`, `helpers/*`)
├── setup/                  # app startup and handlers (debug, analytics, env)
├── stylesheets/            # platform-specific styles (`mobile/` and `web/` inside)
├── utils/                  # helpers (auth, state, track helpers, icons)
└── interfaces/             # typeScript interfaces used across the app
```

## Roadmap

We're constantly improving Flashsound. Here's what's coming:

### High Priority 🔥
- [ ] **Download Button** - Cache tracks for offline playback
- [ ] **Sharing Links** - Share songs with embedded players
- [ ] **Dynamic Backgrounds** - Background colors adapt to album art
- [ ] **Queue System** - Build and manage playback queues
- [ ] **Shuffle Mode** - Randomize playback order

### Core Features 🎯
- [ ] **User Accounts** - Save preferences and playlists across devices
- [ ] **Playlists** - Create, edit, and organize playlists
- [ ] **Favorites/Liked Songs** - Quick access to your favorite tracks
- [ ] **Artist Pages** - Browse by artist with discography
- [ ] **Album Pages** - Full album views with track listings

### Enhanced Experience ✨
- [ ] Repeat modes (off, one, all)
- [ ] Sleep timer
- [ ] Picture-in-picture mode
- [ ] Import local music library
- [ ] Crossfade between tracks
- [ ] Recently played history
- [ ] Lyrics display (synced if available)
- [ ] Custom themes/skins
- [ ] Keyboard shortcuts

### Power User Features 💪
- [ ] Playback speed control
- [ ] Equalizer/audio effects
- [ ] Chromecast/AirPlay support
- [ ] Audio visualizer
- [ ] ReplayGain normalization
- [ ] Gapless playback

### Backend (Flacend) 🔧
- [ ] Song upload with metadata extraction
- [ ] Genre browsing
- [ ] API rate limiting
- [ ] Recommendations engine
- [ ] Radio mode
- [ ] Play count statistics
- [ ] Admin panel

## Contribution

Want to contribute? Hell yeah!  🎉

This project is in active development and we welcome contributions of all kinds. Anything would help, including:
- Bug fixes
- New features from the roadmap
- Documentation improvements
- UI/UX enhancements
- Tests

**To contribute:**

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please make sure your code follows my existing style and is documented properly. TypeScript types are required for all new code.

## Acknowledgments

Built with ❤️ using:
- [React Native](https://reactnative.dev/) - The goats of cross-platform development.
- [Expo](https://expo.dev/) - Making React Native development accessible.
- [Lucide](https://lucide.dev/) - Thanks lucide! These are some clean ass icons.

---

<p align="center">
    Made with 🎵 by <a href="https://github.com/gamerjamer43">gamerjamer43</a>
</p>

<p align="center">
    <sub>Stream responsibly. License your songs, and make sure to support your favorite artists.</sub>
</p>
