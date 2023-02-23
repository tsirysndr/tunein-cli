# TuneIn CLI

A command line interface for [TuneIn Radio](https://tunein.com).<br />
You can search for stations, play them, and see what's currently playing.

## 🚚 Installation

Compile from source:
```bash
git clone https://github.com/tsirysndr/tunein-cli
cd tunein-cli
cargo install --path .
```

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


## 📝 License
[MIT](LICENSE)
