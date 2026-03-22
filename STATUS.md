# Void — Development Status

## Completed

### Phase 0 — Scaffolding
- [x] Cargo.toml, .gitignore, LICENSE (MIT), README.md
- [x] Full module structure (38 source files across 9 modules)
- [x] main.rs, app.rs, cargo check/build/run pass

### Phase 1 — Canvas Foundation
- [x] Pan/zoom (middle-drag, Ctrl+scroll, trackpad pinch, keyboard)
- [x] TSTransform GPU zoom (egui set_transform_layer — Horizon pattern)
- [x] Viewport math (screen↔canvas transforms)
- [x] TerminalPanel with drag, resize, z-order, close, rename
- [x] Viewport culling, minimap, dot grid, sidebar
- [x] Snap guides (alignment lines when dragging panels near each other)

### Phase 2 — Terminal Integration
- [x] portable-pty (ConPTY/Unix PTY) with reader thread + EventProxy
- [x] alacritty_terminal Term with VT parsing, 10k line scrollback
- [x] Terminal renderer (20pt monospace, background batching, cursor shapes)
- [x] Keyboard input mapping (text, Ctrl+key, Alt+key, arrows, F-keys, paste)
- [x] PTY resize on panel resize
- [x] Multiple simultaneous terminals per workspace
- [x] Terminal scroll (scrollback via scroll wheel when focused)
- [x] Text selection (click-and-drag, auto-copy to clipboard)

### Phase 3 — UI & Workspaces
- [x] Independent workspaces (each has own panels, viewport, cwd)
- [x] Folder picker (rfd) to create workspace with working directory
- [x] Sidebar: workspaces with nested terminal dropdowns
- [x] Command palette (Ctrl+Shift+P) with fuzzy search
- [x] Panel context menu, rename dialog (F2)
- [x] Focus cycling (Ctrl+Shift+]/[)
- [x] Zoom to fit (Ctrl+Shift+0)
- [x] Status bar (zoom%, pointer coordinates)
- [x] Minimal dark theme

## Known Issues

- [ ] Text pixelated at low zoom (TSTransform upscales 20pt glyphs — egui 0.30 limitation)
- [ ] Text selection UX needs polish (no double-click word select, no Ctrl+C copy shortcut)
- [ ] No mouse reporting to PTY (TUI apps like vim/htop don't get mouse events)
- [ ] Terminal doesn't feel fully native (no cursor blink, no bell, no URL detection)

## Next Up

### Phase 4 — Terminal Quality
- [ ] Mouse reporting modes (1000/1002/1003/1006) — route mouse to PTY for TUI apps
- [ ] Double-click word select, triple-click line select
- [ ] Ctrl+C / Ctrl+V clipboard integration (not just drag-select auto-copy)
- [ ] Cursor blinking animation
- [ ] Bell notification (visual flash or sound)
- [ ] URL detection + clickable links
- [ ] Scrollbar widget on terminal panel

### Phase 5 — Persistence & Config
- [ ] Save/load workspace state (panel positions, sizes, cwd)
- [ ] Configuration file (void.toml) — font size, theme, keybindings
- [ ] Hot-reload config
- [ ] Remember window size/position

### Phase 6 — Polish
- [ ] Panel animations (smooth open/close)
- [ ] Auto-layout (grid arrange, row arrange)
- [ ] Tabs within panels (multiple shells in one panel)
- [ ] Search in terminal output (Ctrl+Shift+F)
- [ ] Split view (horizontal/vertical split within panel)

### Phase 7+ — See PRD.md Section 23
- [ ] Theming system (multiple themes)
- [ ] Plugin system
- [ ] SSH integration
- [ ] SIXEL image support
- [ ] Documentation site

## Build

```bash
export PATH="/c/msys64/mingw64/bin:$HOME/.cargo/bin:$PATH"
cargo run
```
