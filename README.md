# ytdl-gui

A minimal, modern desktop GUI to download YouTube videos and playlists as
audio or video, built with [Tauri](https://tauri.app) and powered by
[yt-dlp](https://github.com/yt-dlp/yt-dlp). Works as both a **portable**
executable and an **installable** app on **Windows** and **Linux**.

![screenshot](docs/screenshot.png)

## Features

- Audio (mp3/m4a/opus/flac/wav) or video (mp4/mkv/webm) downloads
- Quality selection for both audio and video
- Highest-quality thumbnail embedded as cover art + metadata for audio
- Native folder picker for choosing where files are saved
- Automatic playlist detection, with a choice to grab the whole playlist or
  just the current video
- Live progress bar (title, percentage, speed, ETA)
- Gruvbox Material theme, dark and light, toggle in the top bar
- No telemetry, no bundled ads, no accounts — just a thin GUI over yt-dlp

## Requirements (to run)

The app itself is a small native binary, but it shells out to `yt-dlp` for
the actual downloading, so both of these need to be installed and on your
`PATH`:

- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [ffmpeg](https://ffmpeg.org/download.html)

If either is missing, the app shows a warning banner on startup with
instructions.

## Download

Grab the latest build from the [Releases](../../releases) page:

- **Windows**: `ytdl-gui.exe` (portable, no install) or the `.msi`/`setup.exe`
  installer
- **Linux**: the `.AppImage` (portable, just `chmod +x` and run) or the
  `.deb` package (installable)

## Building from source

Requirements: [Node.js](https://nodejs.org) 18+, [Rust](https://rustup.rs),
and the platform dependencies from the
[Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)
(on Linux this is mainly `webkit2gtk`).

```bash
git clone https://github.com/<your-username>/ytdl-gui.git
cd ytdl-gui
npm install
npm run tauri dev    # run in development
npm run tauri build  # produce portable binary + installer for your OS
```

`tauri build` produces both the portable executable and the installer
package(s) for whichever OS you build on (Windows builds must be made on
Windows, Linux builds on Linux — see `.github/workflows` if you want to
automate cross-OS builds via CI).

## How it works

The Rust backend spawns `yt-dlp` as a subprocess with a custom
`--progress-template`, parses its output live, and streams progress events
to the UI. The folder picker uses Tauri's native dialog plugin. The frontend
is plain HTML/CSS/JS — no framework, no bundler — kept intentionally small.

## License

MIT

---

by **Jvps**
