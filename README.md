# TuneIn CLI

<p>
  <a href="https://flakehub.com/flake/tsirysndr/tunein-cli" target="_blank">
    <img src="https://img.shields.io/endpoint?url=https://flakehub.com/f/tsirysndr/tunein-cli/badge" />
  </a>
  <a href="https://crates.io/crates/tunein-cli" target="_blank">
    <img src="https://img.shields.io/crates/v/tunein-cli.svg" />
  </a>
  <a href="https://flakestry.dev/flake/github/tsirysndr/tunein-cli" target="_blank">
    <img src="https://flakestry.dev/api/badge/flake/github/tsirysndr/tunein-cli" />
  </a>
  <a href="https://crates.io/crates/tunein-cli" target="_blank">
    <img src="https://img.shields.io/crates/dr/tunein-cli" />
  </a>
  <a href="LICENSE" target="./LICENSE">
    <img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-blue.svg" />
  </a>
</p>

A command line interface for [TuneIn Radio](https://tunein.com).<br />
You can search for stations, play them, and see what's currently playing.

![Made with VHS](https://vhs.charm.sh/vhs-4UhZFFRvVAuaZnapZUlp6R.gif)

## 🚚 Installation

Compile from source, without Nix:
```bash
# Install dependencies
brew install protobuf # macOS
sudo apt-get install -y libasound2-dev protobuf-compiler # Ubuntu/Debian
# Compile and install
git clone https://github.com/tsirysndr/tunein-cli
cd tunein-cli
cargo install --path .
```

With Nix:
```bash
git clone https://github.com/tsirysndr/tunein-cli
cd tunein-cli
nix develop --experimental-features "nix-command flakes"
cargo install --path .
```

### macOS/Linux

Using Bash:
```bash
curl -fsSL https://cdn.jsdelivr.net/gh/tsirysndr/tunein@main/install.sh | bash
```

Using [Homebrew](https://brew.sh):

```bash
brew install tsirysndr/tap/tunein
```

Using [Nix](https://nixos.org/nix/):
```bash
nix profile install --experimental-features "nix-command flakes" github:tsirysndr/tunein-cli
```

Or download the latest release for your platform [here](https://github.com/tsirysndr/tunein-cli/releases).

## 📦 Downloads
- `Mac`: arm64: [tunein_v0.2.0_aarch64-apple-darwin.tar.gz](https://github.com/tsirysndr/tunein-cli/releases/download/v0.2.0/tunein_v0.2.0_aarch64-apple-darwin.tar.gz) intel: [tunein_v0.2.0_x86_64-apple-darwin.tar.gz](https://github.com/tsirysndr/tunein-cli/releases/download/v0.2.0/tunein_v0.2.0_x86_64-apple-darwin.tar.gz)
- `Linux`: [tunein_v0.2.0_x86_64-unknown-linux-gnu.tar.gz](https://github.com/tsirysndr/tunein-cli/releases/download/v0.2.0/tunein_v0.2.0_x86_64-unknown-linux-gnu.tar.gz)
## 🚀 Usage
```
USAGE:
    tunein <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    browse    Browse radio stations
    help      Print this message or the help of the given subcommand(s)
    play      Play a radio station
    search    Search for a radio station
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

## API Documentation
[https://buf.build/tsiry/tuneinserverapis/docs/main:tunein.v1alpha1](https://buf.build/tsiry/tuneinserverapis/docs/main:tunein.v1alpha1)

You can start the server locally by running:
```bash
tunein server
```

and then use [Buf Studio](https://studio.buf.build/tsiry/tuneinserverapis?selectedProtocol=grpc-web&target=http%3A%2F%2Flocalhost%3A8090) to make requests to the server

<img src="./api.png" />


## 📝 License
[MIT](LICENSE)
