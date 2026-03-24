# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Void is an infinite canvas terminal emulator — GPU-accelerated, cross-platform, 100% Rust. No Electron, no web stack. Built with eframe/egui + wgpu + alacritty_terminal + portable-pty.

## Commands

```bash
cargo run                    # Dev build + run
cargo check                  # Fast type check
cargo test --locked           # Run all tests
cargo fmt --check             # Format check
cargo clippy --locked --all-targets --all-features -- -D warnings  # Lint
cargo build --release --locked --target x86_64-pc-windows-msvc     # Release (Windows)
```

Binary name is `void`. Linux builds need system deps: `libasound2-dev libudev-dev libwayland-dev libx11-dev libxcursor-dev libxi-dev libxinerama-dev libxkbcommon-dev libxrandr-dev pkg-config`.

## Architecture

```
VoidApp (app.rs)
 └─ workspaces: Vec<Workspace>
     └─ panels: Vec<CanvasPanel>        # enum: Terminal(TerminalPanel) | Webview(...)
         └─ TerminalPanel
             └─ PtyHandle               # PTY process + alacritty_terminal::Term
```

**Rendering pipeline** (in `app.rs::update()`):
1. Handle shortcuts → sync panel titles → route keyboard to focused panel
2. Render sidebar (workspaces/terminals list)
3. Canvas background layer (grid, pan/zoom, status bar)
4. Canvas content layer (panels sorted by z_index, with TSTransform for zoom)
5. Minimap overlay

**Terminal rendering** (`terminal/renderer.rs`): Backgrounds in canvas space (GPU-scaled), text in a separate screen-space layer for crisp rendering at any zoom. Two passes: backgrounds with run-length encoding, then text with per-cell attribute handling.

**Panel placement** (`state/workspace.rs::find_free_position`): Gap-filling algorithm — generates candidates from edge intersections of existing panels, scores by bounding box growth + distance to center. New panels fill L-shaped gaps to form compact layouts.

**PTY lifecycle** (`terminal/pty.rs`): 3 threads per terminal — event thread (OSC/title/bell events), reader thread (PTY output → VTE parser → Term state machine), waiter thread (child exit detection). All communicate via `Arc<Mutex<>>` and `Arc<AtomicBool>`.

## Key modules

- `panel.rs` — `CanvasPanel` enum, uniform interface for all panel types
- `terminal/panel.rs` — `TerminalPanel` struct, chrome rendering (title bar, borders, resize handles), interaction (click/drag/select/scroll), context menu
- `terminal/renderer.rs` — Cell grid rendering, cursor shapes, underline/strikethrough/italic
- `terminal/input.rs` — egui key events → terminal byte sequences (CSI, SS3, control chars, bracketed paste, F1-F20 with modifiers)
- `terminal/pty.rs` — PTY spawn, I/O threads, resize, bell detection, OSC 52 clipboard
- `state/workspace.rs` — Workspace model, panel lifecycle, smart placement
- `state/persistence.rs` — JSON save/load to `~/.local/share/void/layout.json`
- `canvas/viewport.rs` — Pan/zoom math, coordinate transforms (screen ↔ canvas)
- `update.rs` — Auto-update via GitHub releases API, cross-platform install scripts

## Conventions

- Panel fields (`id`, `title`, `position`, `size`, `color`, `z_index`, `focused`) are public on `TerminalPanel` but accessed via `CanvasPanel` wrapper methods in app code.
- The `show()` method on panels returns `PanelInteraction` — the caller (app.rs) processes drag/resize/close actions with snap guides.
- Terminal input goes through `input::process_input()` → bytes → `pty.write()`. The input system filters Void shortcuts (Ctrl+B/M/G, Ctrl+Shift+T) before forwarding to PTY.
- Scrollback: mouse wheel scrolls history in normal mode, sends arrow keys in alt-screen mode, sends SGR mouse events in mouse mode. Any keyboard input snaps scroll back to bottom.
- Persistence saves layout only (positions, sizes, workspace state). PTY processes are respawned on restore.
