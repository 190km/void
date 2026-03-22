<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/banner-light.svg">
    <img alt="Void" src="assets/banner-dark.svg" width="600">
  </picture>
</p>

<p align="center">
  <strong>An infinite canvas where your terminals float free.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/rust-2021_edition-orange" alt="Rust"></a>
  <a href="#"><img src="https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-brightgreen" alt="Platforms"></a>
  <a href="#"><img src="https://img.shields.io/badge/gpu-wgpu_(Vulkan%2FMetal%2FDX12)-blueviolet" alt="GPU"></a>
</p>

---

<!-- ![demo](assets/demo.gif) -->

> No tabs. No splits. No tiling. Just an infinite 2D surface where you place terminals anywhere — pan and zoom to navigate between them.

Built **entirely in Rust**. Zero web technologies. No Electron, no WebView, no JavaScript runtime. Native performance via wgpu.

## Features

| | |
|---|---|
| **Infinite Canvas** | Pan, zoom, and place terminals anywhere on a boundless 2D surface |
| **GPU Accelerated** | 60fps rendering via wgpu — Vulkan, Metal, and DX12 backends |
| **Real Terminal Emulation** | Powered by alacritty_terminal — full ANSI/VT100, 256-color, truecolor |
| **Workspaces** | Independent canvas views for different contexts, each with their own layout |
| **Command Palette** | `Ctrl+Shift+P` — fuzzy search across all actions |
| **Minimap** | Bird's-eye overview of your entire terminal layout |
| **Keyboard-First** | Every action reachable without a mouse |
| **Cross-Platform** | Windows, Linux, macOS |

## How it works

```
  You ──► Infinite Canvas (pan/zoom) ──► Terminal Panels (drag/resize anywhere)
              GPU-rendered (wgpu)              ↕
                                          alacritty_terminal + portable-pty
                                              ↕
                                          Real shell (bash, zsh, powershell, ...)
```

Each terminal panel is an independent PTY process rendered onto the canvas via the GPU. Panels can be freely dragged, resized, and arranged in any spatial layout you want. Zoom out to see everything, zoom in to focus.

## Quickstart

```bash
# prerequisites: rust toolchain (rustup.rs)
git clone https://github.com/190km/void.git
cd void
cargo run
```

<details>
<summary><strong>Windows — MinGW-w64 setup</strong></summary>

```powershell
# ensure MinGW-w64 is on PATH
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo run
```

</details>

<details>
<summary><strong>Release build (optimized)</strong></summary>

```bash
cargo build --release
# binary at target/release/void
```

</details>

## Keyboard shortcuts

| Action | Shortcut |
|---|---|
| New terminal | `Ctrl+Shift+T` |
| Close terminal | `Ctrl+Shift+W` |
| Command palette | `Ctrl+Shift+P` |
| Rename terminal | `F2` |
| Focus next / prev | `Ctrl+Shift+]` / `[` |
| Toggle sidebar | `Ctrl+B` |
| Toggle minimap | `Ctrl+M` |
| Toggle grid | `Ctrl+G` |
| Zoom in / out | `Ctrl+=` / `-` |
| Reset zoom | `Ctrl+0` |
| Fit all terminals | `Ctrl+Shift+0` |
| Pan canvas | Middle-click drag / Scroll |
| Zoom | `Ctrl+Scroll` / Trackpad pinch |

## Architecture

```
src/
├── app.rs              # main application loop
├── canvas/             # pan, zoom, viewport math, minimap, grid
├── terminal/           # panel rendering, PTY management, input handling
├── sidebar/            # workspace list, session list, quick actions
├── command_palette/    # fuzzy search, command registry
├── state/              # workspace & panel state management
├── theme/              # color palette, fonts
├── config/             # schema, defaults, hot-reload
├── shortcuts/          # keybinding system
└── utils/              # ids, platform detection
```

**Stack:** [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) + [egui](https://github.com/emilk/egui) + [wgpu](https://github.com/gfx-rs/wgpu) + [alacritty_terminal](https://github.com/alacritty/alacritty) + [portable-pty](https://github.com/wez/wezterm/tree/main/pty)

## Roadmap

- [x] Infinite canvas with pan/zoom
- [x] GPU-accelerated rendering (wgpu)
- [x] Real terminal emulation (alacritty_terminal + portable-pty)
- [x] Multiple terminals with independent PTY processes
- [x] Full ANSI color support (16, 256, truecolor)
- [x] Workspaces with viewport persistence
- [x] Command palette with fuzzy matching
- [x] Minimap navigation
- [x] Panel drag, resize, close, rename, focus cycling
- [ ] Session persistence (save/restore on quit)
- [ ] Configuration hot-reload (TOML)
- [ ] Auto-layout engine (tidy, snap, arrange)
- [ ] Built-in theme engine + custom themes
- [ ] Text selection & copy
- [ ] Scrollback interaction
- [ ] Plugin system

See [STATUS.md](STATUS.md) for detailed progress and [PRD.md](PRD.md) for the full roadmap.

## Contributing

Void is open source and contributions are welcome.

```bash
# run in dev mode
cargo run

# check before submitting
cargo clippy
cargo fmt --check
```

If you find a bug or have a feature request, [open an issue](https://github.com/190km/void/issues).

## License

[MIT](LICENSE) — 190km


