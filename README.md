# nesium-cli

NES emulator for terminal with iTerm2 image support, based on [mikai233/nesium](https://github.com/mikai233/nesium).

## Features

- **Terminal-based NES emulator**
- **iTerm2 image support** for game display
- **WASD/JKL controls** (WASD for direction, J=A, K=B, L=Select, ;=Start)
- **60 FPS** limit
- **Cycle-accurate** emulation (based on nesium-core)

## Installation

```bash
# From GitHub
cargo install --git https://github.com/wangjianyuan10/nesium-cli
```

## Usage

```bash
# Run NES ROM
nesium-cli path/to/rom.nes

# Controls:
# WASD: Direction
# J: A button
# K: B button
# L: Select
# ;: Start
# Q: Quit
```

## Dependencies

- [nesium-core](https://github.com/mikai233/nesium) (from GitHub)
- [crossterm](https://crates.io/crates/crossterm) for terminal handling
- [ratatui](https://crates.io/crates/ratatui) for terminal UI
- [clap](https://crates.io/crates/clap) for command-line arguments
- [miniz_oxide](https://crates.io/crates/miniz_oxide) for PNG encoding
- [base64](https://crates.io/crates/base64) for image encoding

## Requirements

- **iTerm2** (for image display)
- **macOS** (tested on macOS)
- **Rust 1.70+**

## License

MIT OR Apache-2.0
