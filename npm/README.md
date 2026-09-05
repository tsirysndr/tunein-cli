![Cover](https://raw.githubusercontent.com/tsirysndr/tunein-cli/main/.github/assets/preview.png)

# TuneIn CLI 📻 🎵 ✨

[![npm](https://img.shields.io/npm/v/tunein-cli.svg)](https://www.npmjs.com/package/tunein-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/tsirysndr/tunein-cli/blob/main/LICENSE)

A command line interface for [TuneIn Radio](https://tunein.com) / [Radio Browser](https://www.radio-browser.info/).<br />
You can search for stations, play them, and see what's currently playing — right from your terminal.

This package installs the prebuilt `tunein` binary from the official [GitHub releases](https://github.com/tsirysndr/tunein-cli/releases).

![Made with VHS](https://vhs.charm.sh/vhs-4UhZFFRvVAuaZnapZUlp6R.gif)

## ✨ Features

- 🔍 Search and play thousands of radio stations from [TuneIn](https://tunein.com) or [Radio Browser](https://www.radio-browser.info/)
- 🎵 Plays all the common Icecast stream formats: **MP3, AAC/AAC+, Ogg Vorbis, FLAC and WAV**
- 🎧 Powerful DSP (**Equalizer, Bass, Treble**) based on the [Rockbox DSP](https://github.com/tsirysndr/rockboxd/tree/master/crates/rockbox-dsp) engine
- 📻 Interactive TUI: browse categories, favourites, resume last station
- 🌈 Real-time audio visualizations: oscilloscope, vectorscope and spectroscope
- 🖥️ OS media controls integration (play/pause/volume from your keyboard's media keys)
- 🛰️ Built-in gRPC server, installable as a systemd service
- 🌐 Embedded web UI with a GraphQL API (`tunein web`) — search, browse and listen from your browser

## 🚚 Installation

Install globally:

```bash
npm install -g tunein-cli
```

Or run it directly without installing:

```bash
npx tunein-cli search "BBC Radio 1"
```

Both expose the `tunein` command:

```bash
tunein --version
```

## 🚀 Usage

```
USAGE:
    tunein <SUBCOMMAND>

OPTIONS:
    -h, --help                   Print help information
    -p, --provider <provider>    The radio provider to use, can be 'tunein' or 'radiobrowser'.
                                 Default is 'tunein' [default: tunein]
    -V, --version                Print version information

SUBCOMMANDS:
    browse    Browse radio stations
    help      Print this message or the help of the given subcommand(s)
    play      Play a radio station
    search    Search for a radio station
    server    Start the server
    service   Manage systemd service for tunein-cli server
    web       Start the web UI & GraphQL API server
```

Search for a radio station:

```bash
tunein search "BBC Radio 1"
```

Result:

```
BBC Radio 1 | The best new music | id: s24939
BBC Radio 1Xtra | Remi Burgz | id: s20277
```

Play a radio station:

```bash
tunein play "alternativeradio.us"
# Or by station ID
tunein play s221580
```

Browse stations interactively:

```bash
tunein browse
```

Start the web UI & GraphQL API:

```bash
tunein web          # listens on http://localhost:8881
tunein web 3000     # custom port
```

## 📦 Supported Platforms

| OS      | Architectures            |
| ------- | ------------------------ |
| macOS   | Apple Silicon, Intel     |
| Linux   | x86_64, aarch64, armv7   |
| FreeBSD | x86_64, aarch64          |
| NetBSD  | x86_64, aarch64          |

Windows is not supported. On other platforms, see the [installation guide](https://github.com/tsirysndr/tunein-cli#-installation) for alternatives (Homebrew, Nix, apt, dnf, AUR, or building from source).

## 🔧 How It Works

On install, a `postinstall` script:

1. Detects your OS and CPU architecture.
2. Downloads the matching prebuilt binary from the [GitHub releases](https://github.com/tsirysndr/tunein-cli/releases) page.
3. Verifies its SHA-256 checksum against the published `.sha256` file.
4. Extracts the `tunein` binary into the package's `bin/` directory.

If the install scripts were skipped (e.g. `npm install --ignore-scripts`), the binary is downloaded automatically on first run instead. The package itself has zero npm dependencies.

## 📚 Documentation

- Full documentation, keyboard shortcuts and equalizer guide: [github.com/tsirysndr/tunein-cli](https://github.com/tsirysndr/tunein-cli)
- gRPC API reference: [buf.build/tsiry/tuneinserverapis](https://buf.build/tsiry/tuneinserverapis/docs/main:tunein.v1alpha1)

## 📝 License

[MIT](https://github.com/tsirysndr/tunein-cli/blob/main/LICENSE)
