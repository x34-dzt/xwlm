# xwlm

A TUI for managing your Wayland monitors. Supports Hyprland, Sway, and River.

[![Crates.io](https://img.shields.io/crates/v/xwlm)](https://crates.io/crates/xwlm)
[![Downloads](https://img.shields.io/crates/d/xwlm)](https://crates.io/crates/xwlm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

![xwlm](assets/xwlm.png)

## About

xwlm lets you arrange, resize, rotate, and toggle your monitors from the terminal. It auto-detects your compositor, reads the current monitor state over Wayland, and writes native config files when you apply changes.

No more hand-editing `monitors.conf`.

## Install

**Quick install:**
```sh
curl -fsSL https://x34-dzt.github.io/xwlm/install.sh | bash
```

**crates.io:**
```sh
cargo install xwlm
```

**From source:**
```sh
git clone https://github.com/x34-dzt/xwlm.git
cd xwlm
cargo build --release
# binary is at target/release/xwlm
```

Then just run `xwlm`. On first launch it'll ask where to save your monitor config.

## Keybindings

| Key | Action |
|-----|--------|
| `Tab` | Switch panel |
| `[` `]` | Switch monitor |
| `Arrow keys` | Move monitor / navigate |
| `Enter` | Apply changes |
| `+` `-` | Adjust scale or zoom |
| `t` | Toggle on/off |
| `r` | Reset positions |
| `q` | Quit |

## Compositor Support

| Compositor | Status | Notes |
|------------|--------|-------|
| Hyprland | Tested | Fully supported and actively tested |
| Sway | Untested | Should work — implements the same `wlr-output-management-unstable-v1` protocol |
| River | Untested | Should work — implements the same protocol. Config persistence uses `wlr-randr` commands |

All compositors share the same Wayland protocol (`zwlr_output_management_v1`) for live monitor changes via [wlx_monitors](https://github.com/x34-dzt/wlx_monitors), a Rust library built for this project. The only differences are in config file format and reload mechanism. If you run into issues on Sway or River, please [open an issue](https://github.com/x34-dzt/xwlm/issues).

## Requirements

- Wayland session (Hyprland, Sway, or River)
- Terminal with Unicode support
- `wlr-randr` (River only, for config persistence)

## License

MIT
