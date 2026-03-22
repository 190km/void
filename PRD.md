# Void — Product Requirements Document

## Infinite Canvas Terminal · 100% Rust · Cross-Platform

**Version:** 0.1.0-draft
**Author:** 190km
**Date:** March 2026
**License:** MIT
**Status:** Pre-development

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Vision & Philosophy](#2-vision--philosophy)
3. [Competitive Landscape](#3-competitive-landscape)
4. [Target Users](#4-target-users)
5. [Technical Architecture Overview](#5-technical-architecture-overview)
6. [Technology Stack](#6-technology-stack)
7. [Repository Structure (Monorepo)](#7-repository-structure-monorepo)
8. [Core Feature Specifications](#8-core-feature-specifications)
   - 8.1 [Infinite Canvas](#81-infinite-canvas)
   - 8.2 [Terminal Emulation](#82-terminal-emulation)
   - 8.3 [Terminal Panels](#83-terminal-panels)
   - 8.4 [Left Sidebar](#84-left-sidebar)
   - 8.5 [Command Palette](#85-command-palette)
   - 8.6 [Workspaces](#86-workspaces)
   - 8.7 [Minimap](#87-minimap)
   - 8.8 [Session Persistence](#88-session-persistence)
   - 8.9 [Auto-Layout Engine](#89-auto-layout-engine)
   - 8.10 [Keyboard Shortcuts System](#810-keyboard-shortcuts-system)
   - 8.11 [Theming & Appearance](#811-theming--appearance)
   - 8.12 [Configuration System](#812-configuration-system)
9. [Platform-Specific Requirements](#9-platform-specific-requirements)
10. [Performance Requirements](#10-performance-requirements)
11. [Data Model & State Management](#11-data-model--state-management)
12. [File & Directory Structure (Crate-Level)](#12-file--directory-structure-crate-level)
13. [Module Architecture (Detailed)](#13-module-architecture-detailed)
14. [Rendering Pipeline](#14-rendering-pipeline)
15. [Input Handling](#15-input-handling)
16. [PTY Management](#16-pty-management)
17. [Serialization & Persistence Format](#17-serialization--persistence-format)
18. [Configuration File Format](#18-configuration-file-format)
19. [Build System & CI/CD](#19-build-system--cicd)
20. [Distribution & Installation](#20-distribution--installation)
21. [Documentation Site (Fumadocs)](#21-documentation-site-fumadocs)
22. [Landing Page / Website](#22-landing-page--website)
23. [Development Phases & Milestones](#23-development-phases--milestones)
24. [Testing Strategy](#24-testing-strategy)
25. [Accessibility](#25-accessibility)
26. [Security Considerations](#26-security-considerations)
27. [Future / Post-v1 Features](#27-future--post-v1-features)
28. [Open Questions & Decisions](#28-open-questions--decisions)
29. [Appendix A: Cargo.toml Templates](#appendix-a-cargotoml-templates)
30. [Appendix B: Key Data Structures](#appendix-b-key-data-structures)
31. [Appendix C: Keyboard Shortcuts Reference](#appendix-c-keyboard-shortcuts-reference)
32. [Appendix D: ANSI/VT Escape Sequences to Support](#appendix-d-ansivt-escape-sequences-to-support)
33. [Appendix E: Competitor Feature Matrix](#appendix-e-competitor-feature-matrix)

---

## 1. Executive Summary

**Void** is an open-source, GPU-accelerated, cross-platform desktop application that reimagines the terminal experience as an infinite spatial canvas. Instead of tabs, splits, or tiled panes, users place terminal sessions anywhere on a boundless 2D surface, freely panning and zooming to navigate between them.

Built entirely in Rust with zero web technologies, Void targets developers, DevOps engineers, and power users who juggle many terminal sessions simultaneously and want a spatial, visual way to organize their workflow.

**Key differentiators:**

- **100% Rust, zero web stack** — native performance, no Electron, no WebView, no JavaScript runtime
- **GPU-accelerated rendering** via wgpu (Vulkan/Metal/DX12/OpenGL backends)
- **Infinite canvas** with pan, zoom, and minimap — powered by egui's new `Scene` container
- **Cross-platform from day one** — Windows, Linux, macOS (x64 + ARM)
- **Instant terminal spawning** — alacritty_terminal for VT parsing + portable-pty for cross-platform PTY
- **Session persistence** — layout, scroll position, working directory, and terminal history survive restarts
- **Open source (MIT)** — community-first, designed for contribution

---

## 2. Vision & Philosophy

### 2.1 The Problem

Modern developers routinely work with 5–30+ terminal sessions: frontend dev server, backend server, database CLI, Docker logs, SSH sessions, file watchers, test runners, git operations. Current terminal emulators force these into:

- **Tabs** — hidden, forgotten, context-switching hell
- **Splits/tiles** — constrained to screen real estate, rigid grid layouts
- **Multiple windows** — lost behind other apps, scattered across virtual desktops

None of these approaches give you a *spatial mental model*. You can't glance at your workspace and see everything at once. You can't zoom out to get an overview, then zoom into a specific session.

### 2.2 The Solution

Void treats terminal sessions like sticky notes on an infinite whiteboard. You place them where they make sense to you. Related terminals cluster together. You pan and zoom like a map. Your brain builds a spatial memory: "the database stuff is over to the left, the frontend is top-right, the deploy scripts are bottom-center."

### 2.3 Design Principles

1. **Spatial over sequential** — position conveys meaning, not tab order
2. **Performance is a feature** — 60fps even with 30+ terminals open, GPU-rendered, instant spawn
3. **Zero configuration required** — beautiful and functional out of the box, deeply configurable for power users
4. **Native, not web** — respect the OS, use native rendering, feel like a first-class citizen on every platform
5. **Keyboard-first** — every action reachable without a mouse, but the mouse experience is equally polished
6. **Transparent persistence** — close Void, reopen it, everything is exactly where you left it
7. **Minimalism with depth** — simple surface, powerful when you dig in

### 2.4 Name & Brand

- **Name:** Void
- **Tagline:** "Where your terminals float free."
- **Aesthetic:** Dark by default (pure black canvas `#000000`), monospace typography, subtle neon accent colors (cyan/electric blue), minimal chrome
- **Logo direction:** Stylized "V" suggesting a vortex or portal into infinite space
- **Repo:** `github.com/<user>/void`
- **Domains to check:** `void.sh`, `void.dev`, `void.rs`, `voidterm.dev`

---

## 3. Competitive Landscape

### 3.1 Direct Competitors

| Product | Platform | Stack | Canvas? | Open Source? | Notes |
|---------|----------|-------|---------|-------------|-------|
| **Finite** | macOS only | Swift + Ghostty | Yes (infinite canvas) | No (closed source) | The original inspiration. macOS-only, not Rust, closed source. Beautiful but locked to Apple ecosystem. |
| **Horizon** | Win/Linux/Mac | Rust + egui + wgpu + alacritty_terminal | Yes (infinite canvas) | Yes (MIT) | Closest existing alternative. Uses same tech stack we're targeting. However, Void aims for a cleaner UX, better sidebar, and a different design philosophy. |

### 3.2 Indirect Competitors

| Product | Model | Why it's not enough |
|---------|-------|-------------------|
| **Alacritty** | Single-window GPU terminal | No canvas, no multi-session, no spatial |
| **WezTerm** | Tabs + splits + multiplexer | Rich features but traditional layout model |
| **Kitty** | Tabs + splits + GPU | Powerful but not spatial |
| **iTerm2** | macOS tabs + splits | macOS only, traditional |
| **Windows Terminal** | Tabs + profiles | Traditional model |
| **tmux / Zellij** | Terminal multiplexers | Text-mode tiling, no GUI canvas |

### 3.3 Why Build Void When Horizon Exists?

Horizon proves the concept works. Void aims to improve on it in specific ways:

1. **Cleaner, more opinionated UX** — less config surface, more "it just works"
2. **Proper left sidebar** — workspace navigation, session list, quick actions
3. **Better visual design** — more polish on the default theme, smoother animations
4. **Different community focus** — docs-first, contributor-friendly, Fumadocs website
5. **Codebase designed for extensibility** — plugin-friendly architecture from the start

---

## 4. Target Users

### 4.1 Primary Persona: "The Session Juggler"

- Senior developer or DevOps engineer
- Runs 10–30 terminal sessions daily
- Uses tmux/screen today but wants something visual
- Works across SSH, Docker, local dev servers, databases
- Values speed and keyboard shortcuts
- Uses Linux or macOS primarily, occasionally Windows

### 4.2 Secondary Persona: "The Visual Organizer"

- Full-stack developer
- Likes spatial/visual tools (Figma, Miro, tldraw)
- Wants to see all their work contexts at once
- Prefers organization over raw speed
- Frequently switches between projects

### 4.3 Tertiary Persona: "The Curious Tinkerer"

- Open source enthusiast / Rust learner
- Wants to contribute or fork
- Values clean code, good docs, modular architecture

---

## 5. Technical Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Void Application                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────┐  ┌──────────────────────────────────────────┐    │
│  │          │  │            Canvas Area                    │    │
│  │  Left    │  │  ┌──────────────────────────────────┐    │    │
│  │  Sidebar │  │  │       egui::Scene                 │    │    │
│  │          │  │  │   (pan + zoom container)          │    │    │
│  │  - Work- │  │  │                                   │    │    │
│  │    spaces│  │  │   ┌─────────┐    ┌─────────┐     │    │    │
│  │  - Term  │  │  │   │TermPanel│    │TermPanel│     │    │    │
│  │    list  │  │  │   │  (PTY)  │    │  (PTY)  │     │    │    │
│  │  - Quick │  │  │   └─────────┘    └─────────┘     │    │    │
│  │    actions│ │  │                                   │    │    │
│  │          │  │  │   ┌─────────┐                     │    │    │
│  │          │  │  │   │TermPanel│     ┌────────┐     │    │    │
│  │          │  │  │   │  (PTY)  │     │Minimap │     │    │    │
│  │          │  │  │   └─────────┘     └────────┘     │    │    │
│  │          │  │  │                                   │    │    │
│  │          │  │  └──────────────────────────────────┘    │    │
│  └──────────┘  └──────────────────────────────────────────┘    │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      Core Systems                               │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────┐    │
│  │ PTY Manager  │  │ State Manager │  │  Config Manager  │    │
│  │ (portable-pty│  │ (persistence, │  │  (TOML config,   │    │
│  │  + alacritty │  │  undo/redo,   │  │   hot-reload)    │    │
│  │  _terminal)  │  │  workspaces)  │  │                  │    │
│  └──────────────┘  └───────────────┘  └──────────────────┘    │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                     Platform Layer                              │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  eframe (winit + wgpu)                                    │  │
│  │  Cross-platform window management + GPU rendering         │  │
│  │  Backends: Vulkan (Linux) · Metal (macOS) · DX12 (Win)   │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 5.1 Layer Breakdown

1. **Platform Layer** — `eframe` (wraps `winit` for windowing + `wgpu` for GPU). Provides the window, event loop, and GPU context. Cross-platform out of the box.

2. **Core Systems** — PTY lifecycle management, application state (panels, positions, workspaces), configuration loading/saving, persistence.

3. **UI Layer** — egui immediate-mode GUI. The canvas uses `egui::Scene` for pan/zoom. The sidebar, command palette, and minimap are standard egui widgets.

4. **Terminal Layer** — Each panel runs an `alacritty_terminal::Term` instance for VT parsing/grid management, connected to a `portable-pty` PTY pair for process I/O.

---

## 6. Technology Stack

### 6.1 Core Dependencies

| Crate | Version | Purpose | License |
|-------|---------|---------|---------|
| `eframe` | latest | Window + GPU context (winit + wgpu) | MIT/Apache-2.0 |
| `egui` | latest (must include `Scene`) | Immediate-mode GUI framework | MIT/Apache-2.0 |
| `wgpu` | (via eframe) | GPU abstraction (Vulkan/Metal/DX12/GL) | MIT/Apache-2.0 |
| `alacritty_terminal` | latest | VT100/VT220/xterm parser + terminal grid | Apache-2.0 |
| `portable-pty` | 0.9.x | Cross-platform PTY (Unix PTY + Windows ConPTY) | MIT |
| `serde` + `serde_json` / `toml` | latest | Serialization for config + persistence | MIT/Apache-2.0 |
| `directories` | latest | XDG/platform-specific config/data paths | MIT/Apache-2.0 |
| `notify` | latest | File watcher for config hot-reload | CC0-1.0 |
| `log` + `env_logger` | latest | Logging | MIT/Apache-2.0 |
| `anyhow` | latest | Error handling | MIT/Apache-2.0 |
| `uuid` | latest | Unique IDs for panels/workspaces | MIT/Apache-2.0 |
| `chrono` | latest | Timestamps for session data | MIT/Apache-2.0 |

### 6.2 Why egui + eframe?

After evaluating the major Rust GUI options:

- **egui/eframe** — Immediate mode, GPU-accelerated via wgpu, mature, cross-platform, has `egui::Scene` (pannable/zoomable container added in 2025). Perfect for canvas + sidebar layout. Large community, active development by emilk + Rerun team.
- **Iced** — Elm-like retained mode. Good but `Scene`-equivalent would need to be built from scratch. Less mature for our specific "canvas + sidebar" use case.
- **Slint** — Declarative DSL. GPL license for open source (viral). Would work technically but license is restrictive.
- **Dioxus native** — Still too young for native desktop. React-like API doesn't fit immediate-mode canvas rendering.

### 6.3 Why `egui::Scene`?

egui recently added `egui::Scene` — a built-in pannable, zoomable container that can hold arbitrary widgets and UI elements. This is *exactly* what we need for the infinite canvas. Key properties:

- Pan with middle mouse button or two-finger drag
- Zoom with Ctrl+scroll or pinch
- Contains full egui widgets inside (so terminal panels are real egui widgets, not custom render targets)
- Configurable zoom range (e.g., 0.1x to 4.0x)
- Performance: only visible content is rendered (viewport culling is handled by egui)

### 6.4 Why alacritty_terminal + portable-pty?

**alacritty_terminal** is the terminal emulation library extracted from Alacritty. It provides:

- Full VT100/VT220/xterm escape sequence parsing
- Terminal grid (cells, rows, columns) with scrollback buffer
- Selection handling (mouse selection of text)
- URL detection
- Color management (256-color, truecolor)
- Battle-tested in Alacritty (one of the most-used terminals)

**portable-pty** (from the WezTerm project) provides:

- Cross-platform PTY abstraction (Unix PTY on Linux/macOS, ConPTY on Windows)
- Process spawning into PTY
- PTY resizing
- Read/write streams for PTY I/O
- MIT licensed, 4.6M+ downloads

Together, they give us a complete terminal emulation stack without writing any VT parsing or PTY management from scratch.

---

## 7. Repository Structure

A standard Rust project. The documentation site (Fumadocs) will be added later in a separate `site/` directory or a dedicated repository.

```
void/
├── Cargo.toml                         # Project manifest
├── Cargo.lock
├── src/
│   ├── main.rs                        # Entry point
│   ├── app.rs                         # eframe::App implementation
│   ├── canvas/                        # Infinite canvas module
│   │   ├── mod.rs
│   │   ├── scene.rs                   # egui::Scene wrapper
│   │   ├── viewport.rs               # Viewport/camera management
│   │   ├── minimap.rs                # Minimap rendering
│   │   ├── grid.rs                   # Optional background grid
│   │   └── layout.rs                 # Auto-layout algorithms
│   ├── terminal/                      # Terminal emulation module
│   │   ├── mod.rs
│   │   ├── panel.rs                   # TerminalPanel widget
│   │   ├── pty.rs                     # PTY management
│   │   ├── renderer.rs               # Terminal grid → egui rendering
│   │   ├── input.rs                   # Keyboard input → PTY
│   │   ├── selection.rs              # Text selection handling
│   │   └── colors.rs                 # Color mapping
│   ├── sidebar/                       # Left sidebar module
│   │   ├── mod.rs
│   │   ├── workspace_list.rs
│   │   ├── session_list.rs
│   │   └── quick_actions.rs
│   ├── command_palette/               # Command palette overlay
│   │   ├── mod.rs
│   │   ├── commands.rs               # Command registry
│   │   └── fuzzy.rs                  # Fuzzy matching
│   ├── state/                         # Application state
│   │   ├── mod.rs
│   │   ├── workspace.rs              # Workspace data model
│   │   ├── panel_state.rs            # Panel positions, sizes
│   │   └── persistence.rs            # Save/load state
│   ├── config/                        # Configuration
│   │   ├── mod.rs
│   │   ├── schema.rs                 # Config struct (serde)
│   │   ├── defaults.rs               # Default values
│   │   └── hot_reload.rs             # File watcher
│   ├── theme/                         # Visual theming
│   │   ├── mod.rs
│   │   ├── colors.rs                 # Color palette definitions
│   │   ├── fonts.rs                  # Font loading + management
│   │   └── builtin.rs               # Built-in theme definitions
│   ├── shortcuts/                     # Keybinding system
│   │   ├── mod.rs
│   │   └── default_bindings.rs
│   └── utils/                         # Shared utilities
│       ├── mod.rs
│       ├── id.rs                     # UUID generation helpers
│       └── platform.rs              # Platform detection + OS-specific helpers
├── assets/                            # Bundled assets
│   └── fonts/                         # Default monospace fonts
├── build.rs                           # Build script (embed assets, set version)
├── .github/
│   └── workflows/
│       ├── ci.yml                     # Lint + test + build (all platforms)
│       └── release.yml               # cargo-dist: build + upload binaries
├── dist.toml                          # cargo-dist configuration
├── .gitignore
├── LICENSE                            # MIT
├── README.md
└── CONTRIBUTING.md
```

### 7.1 Development Commands

```bash
# Run in dev mode
cargo run

# Build release binary
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings
cargo fmt --check

# Clean
cargo clean
```

### 7.2 Future: Documentation Site

The Fumadocs (Next.js) documentation site and landing page will be added later (Phase 9) in a `site/` directory at the root of this repo, or in a separate repository. No Turborepo or Node.js tooling is needed until then.

---

## 8. Core Feature Specifications

### 8.1 Infinite Canvas

The canvas is the heart of Void. It is an unbounded 2D surface where terminal panels live.

#### 8.1.1 Pan

- **Middle mouse button drag** — primary pan method
- **Two-finger trackpad drag** — natural scrolling (macOS + Linux Wayland)
- **Spacebar + left click drag** — alternative (Figma-style)
- Pan is smooth with momentum/inertia (configurable)
- No hard boundaries — canvas extends infinitely in all directions

#### 8.1.2 Zoom

- **Ctrl + scroll wheel** — zoom in/out centered on cursor position
- **Trackpad pinch** — natural zoom gesture
- **Keyboard:** `Ctrl + =` zoom in, `Ctrl + -` zoom out, `Ctrl + 0` reset to 100%
- Zoom range: **10% to 400%** (configurable)
- Zoom is smooth with easing
- Current zoom level displayed in bottom-right corner (or minimap)
- At extreme zoom-out (< 25%), terminal text becomes unreadable — panels show a simplified "card" view with just the title and a colored border

#### 8.1.3 Canvas Background

- Default: pure black `#000000` (the "void")
- Optional subtle dot grid pattern (toggleable, like Figma)
- Grid snapping (optional): panels can snap to a grid when dragged
- Background color configurable in theme

#### 8.1.4 Viewport Culling

Only terminal panels that intersect the visible viewport are rendered. Panels off-screen are skipped entirely. This is critical for performance with 30+ terminals.

**Implementation:** Each frame, compute the visible rect in canvas space (accounting for pan + zoom). For each panel, check AABB intersection with visible rect. Only call the terminal renderer for intersecting panels.

#### 8.1.5 egui::Scene Integration

```rust
// Pseudocode for canvas rendering
egui::Scene::new()
    .zoom_range(0.1..=4.0)
    .show(ui, &mut self.scene_rect, |scene_ui| {
        // Render all visible terminal panels
        for panel in self.visible_panels(scene_rect) {
            panel.render(scene_ui);
        }
        
        // Render minimap overlay
        self.minimap.render(scene_ui);
    });
```

### 8.2 Terminal Emulation

Each terminal panel runs a fully functional terminal emulator capable of running shells, TUI applications (vim, htop, less), and handling all standard escape sequences.

#### 8.2.1 VT Parsing (alacritty_terminal)

We use `alacritty_terminal::Term` as the terminal state machine. It handles:

- **Character encoding:** UTF-8 with proper wide character (CJK) support
- **Escape sequences:** Full VT100/VT220/xterm compatibility
  - CSI sequences (cursor movement, colors, scrolling regions)
  - OSC sequences (window title, hyperlinks, clipboard)
  - DCS sequences (SIXEL images — future)
  - SGR attributes (bold, italic, underline, strikethrough, colors)
- **Color support:**
  - 16 ANSI colors (with configurable palette)
  - 256 indexed colors
  - 24-bit truecolor (RGB)
- **Scrollback buffer:** configurable, default 10,000 lines, max 100,000
- **Alternate screen buffer:** for TUI apps (vim, less, htop)
- **Mouse reporting:** modes 1000, 1002, 1003, 1006 (SGR extended)
- **Bracketed paste mode**
- **Selection:** character, word, line, rectangular (block) selection
- **URL detection:** auto-detect URLs in terminal output, clickable

#### 8.2.2 PTY Management (portable-pty)

Each terminal panel owns a PTY pair:

```rust
// Pseudocode for PTY creation
let pty_system = portable_pty::native_pty_system();
let pty_pair = pty_system.openpty(PtySize {
    rows: 24,
    cols: 80,
    pixel_width: 0,
    pixel_height: 0,
})?;

let cmd = CommandBuilder::new(shell_path); // e.g., /bin/bash, /bin/zsh, pwsh.exe
cmd.cwd(working_directory);
cmd.env("TERM", "xterm-256color");
cmd.env("COLORTERM", "truecolor");

let child = pty_pair.slave.spawn_command(cmd)?;
```

PTY I/O runs on a dedicated thread per terminal:

1. **Reader thread** — reads bytes from PTY master, feeds to `alacritty_terminal::Term`
2. **Writer** — keyboard input from egui → write to PTY master
3. **Resize** — when panel is resized on canvas, send new PTY size

#### 8.2.3 Terminal Rendering

The terminal grid from `alacritty_terminal` must be rendered into egui. This is the most performance-sensitive part.

**Approach: Custom egui painting**

```rust
// For each visible cell in the terminal grid:
// 1. Draw background rect (if cell has a background color != default)
// 2. Draw text glyph (using egui's font system or custom glyph atlas)
//
// Optimization: batch cells with the same style into single draw calls
// Optimization: only re-render dirty regions (cells that changed since last frame)
```

**Font rendering:**
- Use a monospace font (default: bundled JetBrains Mono or similar)
- Font rasterization via egui's built-in text rendering
- Cell size = font advance width × font line height
- Bold, italic, bold-italic variants loaded separately
- Ligature support: optional (configurable, off by default for correctness)

**Color handling:**
- Map alacritty_terminal's `Color` enum to egui's `Color32`
- Support named colors (ANSI 0-15), indexed colors (16-255), and RGB truecolor
- Named colors customizable in theme (so Catppuccin, Dracula, etc. work)

### 8.3 Terminal Panels

A terminal panel is the visual container for a single terminal session on the canvas.

#### 8.3.1 Panel Structure

```
┌─[Title Bar]──────────────────────────── [×]─┐
│  ~/projects/void (zsh)                       │
├──────────────────────────────────────────────┤
│                                              │
│  $ cargo build --release                     │
│     Compiling void v0.1.0                    │
│     Finished release [optimized]             │
│  $ _                                         │
│                                              │
│                                              │
└──────────────────────────────────────────────┘
```

#### 8.3.2 Title Bar

- Shows: working directory (abbreviated) + shell name
- Optional: custom user-set title (via OSC escape or manual rename)
- Close button (×) on hover
- Drag to move panel on canvas
- Double-click title bar to rename

#### 8.3.3 Resizing

- Drag edges or corners to resize
- Minimum size: 40 cols × 10 rows
- No maximum size (you can make a terminal huge and zoom out)
- Resizing sends PTY resize signal (SIGWINCH on Unix, resize on ConPTY)
- Resize handles visible on hover (subtle dots or lines at edges/corners)

#### 8.3.4 Visual States

- **Focused** — bright border (accent color), receives keyboard input
- **Unfocused** — dimmed border, still visible and updating
- **Selected (multi-select)** — highlighted for group operations
- **Exited** — process has exited. Show "[exited]" in title bar. Panel remains visible until manually closed. Dim the terminal content slightly.
- **Disconnected** — PTY error. Show error state, offer "Restart" button.

#### 8.3.5 Panel Actions (Right-click Context Menu)

- Rename
- Duplicate (spawn new terminal with same CWD)
- Close
- Move to workspace...
- Set color tag (for visual organization)
- Copy all output
- Clear scrollback
- Reset terminal
- Split (spawn adjacent terminal — optional convenience)

#### 8.3.6 Panel Z-Order

Panels can overlap. Clicking a panel brings it to the front. Z-order is persisted.

### 8.4 Left Sidebar

The sidebar is a fixed-width panel on the left side of the window (outside the canvas). It provides navigation and workspace management.

#### 8.4.1 Layout

```
┌──────────────────────┐
│  ≡  VOID             │  ← App title + collapse toggle
├──────────────────────┤
│  WORKSPACES          │
│  ● Default        ✦  │  ← Active workspace indicator
│  ○ Backend            │
│  ○ DevOps             │
│  ○ Personal           │
│  + New workspace      │
├──────────────────────┤
│  TERMINALS (5)       │
│  ▸ ~/proj/void (zsh) │  ← Click to focus/zoom to panel
│  ▸ ~/proj/api (zsh)  │
│  ▸ docker logs       │
│  ▸ ssh prod-01       │
│  ▸ htop              │
├──────────────────────┤
│  QUICK ACTIONS       │
│  ⊕ New Terminal      │
│  ⚙ Settings          │
│  ⌨ Shortcuts         │
│  ? Help              │
└──────────────────────┘
```

#### 8.4.2 Sidebar Behavior

- Default width: 240px
- Collapsible: toggle with `Ctrl + B` or click the ≡ icon
- When collapsed: shows only icons, width ~48px
- Resizable: drag right edge to adjust width
- Sections are collapsible independently

#### 8.4.3 Workspace List

- Shows all workspaces with active indicator
- Click to switch workspace (canvas pans to that workspace's viewport)
- Right-click for rename, delete, duplicate, set color
- Drag to reorder

#### 8.4.4 Terminal List

- Shows all terminals in the current workspace
- Click to focus (zoom canvas to that panel)
- Double-click to zoom and focus input
- Shows abbreviated CWD and shell
- Running process name shown if different from shell (e.g., "vim", "cargo build")
- Color-coded dot matching panel's color tag
- Drag to reorder (visual order in list, not canvas position)

### 8.5 Command Palette

A Spotlight/VSCode-style overlay for quick actions.

#### 8.5.1 Activation

- `Ctrl + Shift + P` (or `Cmd + Shift + P` on macOS)

#### 8.5.2 Features

- Fuzzy text matching (no external deps — implement simple fuzzy scorer)
- Shows all available actions with keyboard shortcut hints
- Recent commands at top
- Categories: Terminals, Workspaces, Navigation, Settings, View

#### 8.5.3 Available Commands

```
> New Terminal                    Ctrl+Shift+T
> Close Terminal                  Ctrl+Shift+W
> Rename Terminal                 F2
> Switch Workspace: Default       
> Switch Workspace: Backend       
> Zoom to Fit All                Ctrl+Shift+0
> Zoom to Panel: ~/proj/void     
> Toggle Sidebar                 Ctrl+B
> Toggle Minimap                 Ctrl+M
> Open Settings                  Ctrl+,
> Reset Layout                   
> Auto-Arrange: Grid             
> Auto-Arrange: Rows             
> Auto-Arrange: Columns          
> Clear All Terminals            
> Toggle Fullscreen              F11
```

### 8.6 Workspaces

Workspaces are independent canvas states. Each workspace has its own set of terminal panels at their own positions, its own pan/zoom state, and its own viewport.

#### 8.6.1 Data Model

```rust
struct Workspace {
    id: Uuid,
    name: String,
    color: Option<Color32>,    // Color tag for sidebar
    panels: Vec<PanelId>,      // Panels belonging to this workspace
    viewport: ViewportState,   // Pan offset + zoom level
    created_at: DateTime,
    last_accessed: DateTime,
}
```

#### 8.6.2 Workspace Switching

- Click in sidebar
- Keyboard: `Ctrl + 1` through `Ctrl + 9` for first 9 workspaces
- Command palette
- Smooth animated transition (pan + zoom interpolation between workspace viewports)

#### 8.6.3 Default Behavior

- App starts with one workspace called "Default"
- Terminals created without specifying a workspace go to the active one
- Deleting a workspace offers to move its terminals to another workspace or close them

### 8.7 Minimap

A small overview of the entire canvas, showing all terminal panels as colored rectangles.

#### 8.7.1 Position & Size

- Bottom-right corner of the canvas area
- Default size: 200×150px
- Toggleable: `Ctrl + M`
- Draggable to reposition (snaps to corners)
- Semi-transparent background

#### 8.7.2 Features

- Shows all panels as small colored rectangles (color = panel's color tag or workspace color)
- Shows current viewport as a highlighted rectangle
- Click on minimap to pan canvas to that location
- Drag viewport rectangle on minimap to pan in real time
- Current zoom level displayed below minimap

### 8.8 Session Persistence

Everything is saved and restored automatically.

#### 8.8.1 What is Persisted

- All workspaces (name, color, viewport state)
- All panels (position, size, z-order, color tag, custom title)
- Panel working directory
- Panel scrollback buffer (configurable: off, last N lines, or full)
- Panel environment variables (optional)
- Which panel was focused
- Sidebar state (collapsed, width, section collapse states)
- Window geometry (position, size, maximized state)

#### 8.8.2 When is State Saved

- **On every significant change** — panel move, resize, create, close, workspace switch
- **Periodic autosave** — every 30 seconds (configurable)
- **On clean exit** — full state dump on graceful shutdown
- **Debounced** — rapid changes (e.g., dragging a panel) are debounced to avoid excessive disk I/O

#### 8.8.3 Where is State Stored

Platform-specific data directory:

- **Linux:** `~/.local/share/void/state.json`
- **macOS:** `~/Library/Application Support/com.void.terminal/state.json`
- **Windows:** `%APPDATA%\void\state.json`

Use the `directories` crate for cross-platform path resolution.

#### 8.8.4 State File Format

JSON for human readability and easy debugging. See Appendix B for full schema.

### 8.9 Auto-Layout Engine

Automatically arrange terminal panels in common patterns.

#### 8.9.1 Layout Modes

1. **Grid** — arrange panels in an N×M grid, evenly spaced
2. **Rows** — arrange panels in horizontal rows, left to right
3. **Columns** — arrange panels in vertical columns, top to bottom
4. **Stack** — stack all panels centered, slightly offset (like a card stack)
5. **Cascade** — position panels in a diagonal cascade (like old Windows MDI)

#### 8.9.2 Behavior

- Triggered from command palette, right-click canvas context menu, or keyboard shortcut
- Animated transition from current positions to new layout (ease-in-out, ~300ms)
- Only affects panels in the current workspace
- Does not change panel sizes (except Grid mode which normalizes sizes)
- After auto-layout, panels are freely movable again (layout is a one-shot action, not a constraint)

### 8.10 Keyboard Shortcuts System

#### 8.10.1 Design Philosophy

All shortcuts use `Ctrl + Shift + <key>` prefix to avoid conflicts with terminal programs (which commonly use `Ctrl + C`, `Ctrl + D`, `Ctrl + Z`, etc.).

On macOS, `Ctrl` is replaced with `Cmd` where appropriate.

#### 8.10.2 Input Priority

1. If the command palette is open → palette handles input
2. If a terminal panel is focused → check if it's a Void shortcut (Ctrl+Shift+...) → if yes, Void handles it; if no, pass to terminal PTY
3. If no terminal is focused → Void handles all input

#### 8.10.3 Default Bindings

See Appendix C for the full reference table.

#### 8.10.4 Customization

Keybindings are customizable in the config file (`void.toml`):

```toml
[keybindings]
new_terminal = "Ctrl+Shift+T"
close_terminal = "Ctrl+Shift+W"
command_palette = "Ctrl+Shift+P"
toggle_sidebar = "Ctrl+B"
# ... etc
```

### 8.11 Theming & Appearance

#### 8.11.1 Built-in Themes

1. **Void Dark** (default) — pure black background, white text, cyan accents
2. **Void Light** — white canvas, dark text (for the brave)
3. **Catppuccin Mocha** — popular warm dark theme
4. **Dracula** — another popular dark theme
5. **Nord** — cool blue-gray theme

#### 8.11.2 Theme Structure

```toml
[theme]
name = "void-dark"

[theme.canvas]
background = "#000000"
grid_color = "#1a1a1a"
grid_visible = false

[theme.panel]
background = "#0d0d0d"
border_color = "#333333"
border_color_focused = "#00d4ff"
border_width = 1.0
border_radius = 4.0
title_bar_background = "#1a1a1a"
title_bar_text = "#cccccc"
close_button_hover = "#ff4444"

[theme.terminal]
foreground = "#d4d4d4"
background = "#0d0d0d"
cursor_color = "#00d4ff"
selection_background = "#264f78"
# ANSI colors
black   = "#1e1e1e"
red     = "#f44747"
green   = "#6a9955"
yellow  = "#dcdcaa"
blue    = "#569cd6"
magenta = "#c586c0"
cyan    = "#4ec9b0"
white   = "#d4d4d4"
bright_black   = "#808080"
bright_red     = "#f44747"
bright_green   = "#6a9955"
bright_yellow  = "#dcdcaa"
bright_blue    = "#569cd6"
bright_magenta = "#c586c0"
bright_cyan    = "#4ec9b0"
bright_white   = "#ffffff"

[theme.sidebar]
background = "#0a0a0a"
text = "#cccccc"
text_muted = "#666666"
active_item = "#00d4ff"
hover_background = "#1a1a1a"

[theme.minimap]
background = "#0a0a0a80"  # 50% alpha
viewport_border = "#00d4ff"
panel_default = "#333333"
```

#### 8.11.3 Font Configuration

```toml
[font]
family = "JetBrains Mono"  # or "Fira Code", "Cascadia Code", etc.
size = 13.0                # in points
line_height = 1.2          # multiplier
bold_is_bright = false     # bold text uses bright colors (terminal convention)
ligatures = false          # enable font ligatures
```

### 8.12 Configuration System

#### 8.12.1 Config File Location

- **Linux:** `~/.config/void/void.toml`
- **macOS:** `~/Library/Application Support/com.void.terminal/void.toml`
- **Windows:** `%APPDATA%\void\void.toml`

#### 8.12.2 Hot Reload

Configuration file is watched with the `notify` crate. Changes are applied immediately without restarting Void. A subtle toast notification appears: "Configuration reloaded."

Exceptions that require restart:
- GPU backend changes
- Font family changes (requires font atlas rebuild)

#### 8.12.3 Full Config Schema

See Section 18 for the complete TOML schema.

---

## 9. Platform-Specific Requirements

### 9.1 Linux

- **Windowing:** X11 and Wayland (via winit, which supports both)
- **GPU:** Vulkan preferred, OpenGL fallback
- **Shell default:** `$SHELL` environment variable, fallback to `/bin/bash`
- **PTY:** Unix PTY via `portable-pty` (uses `openpty(3)`)
- **Clipboard:** X11 clipboard + primary selection (via `arboard` crate)
- **System dependencies:** None beyond what wgpu needs (Vulkan drivers or Mesa)
- **Packaging:** `.deb`, `.rpm`, `.AppImage`, `.tar.gz`
- **Desktop entry:** Provide `.desktop` file for app launchers

### 9.2 macOS

- **Windowing:** Native Cocoa (via winit)
- **GPU:** Metal (preferred), OpenGL fallback
- **Shell default:** `$SHELL`, fallback `/bin/zsh`
- **PTY:** Unix PTY via `portable-pty`
- **Clipboard:** macOS pasteboard (via `arboard`)
- **Notarization:** Sign + notarize for Gatekeeper (requires Apple Developer account — optional for v1, users can right-click > Open)
- **Packaging:** `.dmg` (drag to Applications), `.pkg`
- **Targets:** `x86_64-apple-darwin` (Intel) + `aarch64-apple-darwin` (Apple Silicon)
- **Universal binary:** Consider `lipo` to create universal binary combining both architectures

### 9.3 Windows

- **Windowing:** Native Win32 (via winit)
- **GPU:** DX12 preferred, Vulkan secondary, DX11 fallback
- **Shell default:** `%COMSPEC%` (usually `cmd.exe`), configurable to PowerShell, pwsh, Git Bash, WSL
- **PTY:** Windows ConPTY via `portable-pty`
  - ConPTY flags to set: `PSEUDOCONSOLE_RESIZE_QUIRK`, `PSEUDOCONSOLE_WIN32_INPUT_MODE`
  - Minimum Windows 10 version 1809 (October 2018 Update) for ConPTY
- **Clipboard:** Win32 clipboard (via `arboard`)
- **Packaging:** `.msi` (installer), `.exe` (standalone portable)
- **Special considerations:**
  - Handle DPI scaling properly (Windows DPI awareness)
  - Support dark/light mode OS theme detection

---

## 10. Performance Requirements

### 10.1 Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Frame rate | ≥60 FPS | With 10 visible terminals actively outputting |
| Terminal spawn | <100ms | Time from "new terminal" action to first shell prompt |
| Canvas pan/zoom | <16ms frame time | During continuous pan/zoom with 30+ panels on canvas |
| Memory per terminal | <20MB | Idle terminal with 10K scrollback |
| Input latency | <10ms | Keypress to character appearing on screen |
| Startup time | <500ms | Cold start to first frame rendered |
| State save | <100ms | Debounced state serialization to disk |
| State load | <200ms | Deserialize state + restore all panels on startup |

### 10.2 Optimization Strategies

1. **Viewport culling** — only render terminals visible in the current viewport
2. **Dirty region tracking** — only re-render terminal cells that changed since last frame
3. **Batched rendering** — batch cells with same style into single draw calls
4. **Async PTY I/O** — PTY reads on separate threads, non-blocking to GUI thread
5. **Lazy scrollback** — don't render scrollback that's not visible
6. **Font atlas caching** — pre-rasterize all glyphs into a texture atlas
7. **State serialization off main thread** — save state on a background thread

### 10.3 Profiling

Include debug mode profiling tools:
- `VOID_PROFILE=1` env var enables frame time overlay
- FPS counter (toggleable, `Ctrl + Shift + F12`)
- egui's built-in inspection panel (toggleable in debug builds)

---

## 11. Data Model & State Management

### 11.1 Core Types

```rust
/// Unique identifier for a terminal panel
type PanelId = Uuid;

/// Unique identifier for a workspace
type WorkspaceId = Uuid;

/// The complete application state
struct AppState {
    workspaces: Vec<Workspace>,
    active_workspace_id: WorkspaceId,
    panels: HashMap<PanelId, PanelState>,
    focused_panel_id: Option<PanelId>,
    sidebar: SidebarState,
    window: WindowState,
    global_settings: GlobalSettings,
}

/// A workspace — an independent canvas view
struct Workspace {
    id: WorkspaceId,
    name: String,
    color: Option<[u8; 4]>,      // RGBA
    panel_ids: Vec<PanelId>,      // Ordered by z-index (back to front)
    viewport: ViewportState,
    created_at: i64,              // Unix timestamp
    last_accessed: i64,
}

/// The viewport (camera) state for a workspace
struct ViewportState {
    pan: [f32; 2],     // x, y offset in canvas coordinates
    zoom: f32,          // 0.1 to 4.0
}

/// A terminal panel's state (serializable)
struct PanelState {
    id: PanelId,
    position: [f32; 2],   // x, y on canvas
    size: [f32; 2],        // width, height in pixels (at 100% zoom)
    title: String,         // Custom title or auto-generated
    shell: String,         // Shell command used to spawn
    cwd: String,           // Current working directory
    color_tag: Option<[u8; 4]>,  // Optional color label
    z_index: u32,
    created_at: i64,
    // Runtime-only fields (not serialized):
    // - term: alacritty_terminal::Term
    // - pty: PtyPair
    // - reader_thread: JoinHandle
}

/// Sidebar UI state
struct SidebarState {
    visible: bool,
    width: f32,
    collapsed_sections: HashSet<String>,
}

/// Window geometry
struct WindowState {
    position: Option<[i32; 2]>,
    size: [u32; 2],
    maximized: bool,
    fullscreen: bool,
}
```

### 11.2 Runtime State (Non-Serialized)

```rust
/// Runtime data for an active terminal panel
struct PanelRuntime {
    term: Arc<Mutex<alacritty_terminal::Term<EventListener>>>,
    pty_master: Box<dyn portable_pty::MasterPty>,
    pty_writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child>,
    reader_thread: JoinHandle<()>,
    dirty: bool,  // Whether the terminal grid has changed since last render
}
```

---

## 12. File & Directory Structure (Crate-Level)

Detailed breakdown of the `src/` directory:

```
src/
├── main.rs                    # Entry point: parse CLI args, setup logging, launch eframe
├── app.rs                     # VoidApp struct implementing eframe::App
│
├── canvas/
│   ├── mod.rs                 # Canvas module: orchestrates Scene + panels + minimap
│   ├── scene.rs               # Wrapper around egui::Scene with custom input handling
│   ├── viewport.rs            # Camera/viewport math: screen ↔ canvas coordinate transforms
│   ├── minimap.rs             # Minimap widget rendering + interaction
│   ├── grid.rs                # Optional background grid rendering
│   └── layout.rs              # Auto-layout algorithms (grid, rows, columns, stack, cascade)
│
├── terminal/
│   ├── mod.rs                 # Terminal module: manages all terminal panels
│   ├── panel.rs               # TerminalPanel struct: combines state + runtime + rendering
│   ├── pty.rs                 # PTY lifecycle: spawn, resize, read/write, cleanup
│   ├── renderer.rs            # Renders alacritty_terminal grid cells to egui painter
│   ├── input.rs               # Maps egui keyboard events to terminal input bytes
│   ├── selection.rs           # Text selection handling (click, drag, word, line)
│   └── colors.rs              # Color mapping: alacritty Color → egui Color32
│
├── sidebar/
│   ├── mod.rs                 # Sidebar module: renders the left panel
│   ├── workspace_list.rs      # Workspace list widget
│   ├── session_list.rs        # Terminal session list widget
│   └── quick_actions.rs       # Quick actions (new terminal, settings, etc.)
│
├── command_palette/
│   ├── mod.rs                 # Command palette overlay
│   ├── commands.rs            # Command registry (all available actions)
│   └── fuzzy.rs               # Fuzzy string matching algorithm
│
├── state/
│   ├── mod.rs                 # AppState struct + state management
│   ├── workspace.rs           # Workspace CRUD operations
│   ├── panel_state.rs         # Panel state management
│   └── persistence.rs         # Save/load to JSON file (debounced, async)
│
├── config/
│   ├── mod.rs                 # Config loading + validation
│   ├── schema.rs              # Config structs (serde + TOML deserialization)
│   ├── defaults.rs            # Default config values
│   └── hot_reload.rs          # File watcher for live config reload
│
├── theme/
│   ├── mod.rs                 # Theme application to egui
│   ├── colors.rs              # Color palette types + conversion
│   ├── fonts.rs               # Font loading, atlas management
│   └── builtin.rs             # Built-in theme definitions (void-dark, catppuccin, etc.)
│
├── shortcuts/
│   ├── mod.rs                 # Shortcut system: register, match, dispatch
│   └── default_bindings.rs    # Default keybinding map
│
└── utils/
    ├── mod.rs
    ├── id.rs                  # UUID generation helpers
    └── platform.rs            # Platform detection + OS-specific helpers
```

---

## 13. Module Architecture (Detailed)

### 13.1 main.rs

```rust
fn main() -> Result<()> {
    // 1. Parse CLI arguments (--config, --version, --help, --log-level)
    // 2. Initialize logging (env_logger)
    // 3. Load configuration from disk (or create default)
    // 4. Create eframe NativeOptions with wgpu backend
    // 5. Launch eframe::run_native("Void", options, app_creator)
}
```

CLI arguments:

- `--config <path>` — custom config file path
- `--version` — print version and exit
- `--log-level <level>` — debug, info, warn, error
- `--reset-state` — start fresh (ignore persisted state)
- `--profile` — enable performance profiling overlay

### 13.2 app.rs — VoidApp

The central application struct implementing `eframe::App`:

```rust
struct VoidApp {
    state: AppState,                          // Serializable state
    panels: HashMap<PanelId, PanelRuntime>,   // Runtime terminal data
    config: AppConfig,                        // Loaded configuration
    sidebar: SidebarWidget,                   // Sidebar UI
    command_palette: CommandPalette,           // Command palette UI
    canvas: CanvasWidget,                     // Canvas + Scene
    config_watcher: Option<ConfigWatcher>,    // Hot-reload file watcher
    save_timer: Instant,                      // Debounce timer for state saves
    
    // Transient UI state
    show_command_palette: bool,
    show_fps: bool,
    toasts: Vec<Toast>,                       // Toast notification queue
}

impl eframe::App for VoidApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 1. Process config hot-reload events
        // 2. Process PTY read events (check for dirty terminals)
        // 3. Handle global keyboard shortcuts
        // 4. Render sidebar (if visible)
        // 5. Render canvas (central panel with egui::Scene)
        // 6. Render command palette overlay (if open)
        // 7. Render toast notifications
        // 8. Save state if debounce timer elapsed
        // 9. Request repaint if any terminal is dirty
    }
}
```

### 13.3 Terminal Rendering Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│ PTY Reader Thread (per terminal)                            │
│                                                             │
│  loop {                                                     │
│    bytes = pty_master.read()                                │
│    term.lock().process(bytes)   // alacritty_terminal       │
│    dirty_flag.set(true)                                     │
│    ctx.request_repaint()        // wake up egui event loop  │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│ GUI Thread (eframe::App::update)                            │
│                                                             │
│  for panel in visible_panels {                              │
│    if panel.dirty {                                         │
│      let term = panel.term.lock()                           │
│      let grid = term.renderable_content()                   │
│      renderer.render_grid(ui, grid, &theme, panel.rect)     │
│      panel.dirty = false                                    │
│    } else {                                                 │
│      // Render cached frame (egui handles this implicitly)  │
│    }                                                        │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
```

---

## 14. Rendering Pipeline

### 14.1 Frame Flow

```
1. eframe calls VoidApp::update(ctx, frame)
2. Process input events (keyboard, mouse, resize)
3. If sidebar visible:
   a. egui::SidePanel::left() → render sidebar widgets
4. egui::CentralPanel::default() → remaining area for canvas
5. Inside central panel:
   a. egui::Scene::new().show() → pannable/zoomable area
   b. Compute visible viewport rect in canvas space
   c. For each panel intersecting viewport:
      i.  Render panel background + border
      ii. Render title bar
      iii. If panel is close enough to read (zoom > ~25%):
           - Lock terminal grid
           - Iterate visible cells
           - Batch draw: backgrounds, then text
      iv. If panel is too far to read (zoom < ~25%):
           - Render simplified "card" view (title + colored rect)
   d. Render minimap overlay (if enabled)
6. If command palette open:
   a. Render full-screen semi-transparent overlay
   b. Render palette input + results
7. Render toast notifications (bottom-right)
8. ctx.request_repaint() if any terminal is dirty
```

### 14.2 Terminal Cell Rendering

```rust
fn render_grid(
    ui: &mut egui::Ui,
    content: RenderableContent,
    theme: &TerminalTheme,
    panel_rect: Rect,
) {
    let painter = ui.painter_at(panel_rect);
    let cell_width = font_advance_width;
    let cell_height = font_line_height;
    
    // Pass 1: Draw background rects (batch by color)
    let mut bg_batches: HashMap<Color32, Vec<Rect>> = HashMap::new();
    for cell in content.display_iter() {
        let bg_color = resolve_color(cell.bg, theme);
        if bg_color != theme.background {
            let rect = cell_rect(cell.point, cell_width, cell_height, panel_rect);
            bg_batches.entry(bg_color).or_default().push(rect);
        }
    }
    for (color, rects) in bg_batches {
        for rect in rects {
            painter.rect_filled(rect, 0.0, color);
        }
    }
    
    // Pass 2: Draw text glyphs (batch by style)
    for cell in content.display_iter() {
        if cell.c != ' ' && cell.c != '\0' {
            let pos = cell_position(cell.point, cell_width, cell_height, panel_rect);
            let fg_color = resolve_color(cell.fg, theme);
            let font_id = resolve_font(cell.flags); // bold, italic, etc.
            painter.text(
                pos,
                Align2::LEFT_TOP,
                cell.c.to_string(),
                font_id,
                fg_color,
            );
        }
    }
    
    // Pass 3: Draw cursor
    if content.cursor.is_visible() {
        let cursor_rect = cell_rect(
            content.cursor.point,
            cell_width, cell_height, panel_rect,
        );
        painter.rect_filled(cursor_rect, 0.0, theme.cursor_color);
    }
}
```

---

## 15. Input Handling

### 15.1 Keyboard Input Flow

```
User presses key
    │
    ▼
eframe/winit captures key event
    │
    ▼
VoidApp::update() receives egui::InputState
    │
    ├── Is command palette open?
    │   YES → palette.handle_input(key) → consume
    │   NO  ↓
    │
    ├── Is it a Void global shortcut? (Ctrl+Shift+...)
    │   YES → dispatch_shortcut(action) → consume
    │   NO  ↓
    │
    ├── Is a terminal panel focused?
    │   YES → terminal_input::key_to_bytes(key, modifiers)
    │         → pty_writer.write(bytes)
    │   NO  → ignore (or handle canvas-level input like arrow keys for pan)
```

### 15.2 Mouse Input Flow

```
Mouse event
    │
    ├── In sidebar area?
    │   YES → sidebar handles (clicks, scrolls, drags)
    │
    ├── In minimap area?
    │   YES → minimap handles (click to pan, drag viewport)
    │
    ├── In canvas area?
    │   │
    │   ├── On panel title bar?
    │   │   YES → drag to move panel
    │   │
    │   ├── On panel resize handle?
    │   │   YES → drag to resize panel
    │   │
    │   ├── On panel terminal area?
    │   │   LEFT CLICK → focus panel, place cursor (if mouse reporting)
    │   │   RIGHT CLICK → context menu
    │   │   MIDDLE CLICK → paste (if configured)
    │   │   SCROLL → scroll terminal scrollback (if focused panel)
    │   │
    │   ├── On canvas background?
    │   │   LEFT CLICK → unfocus all panels
    │   │   RIGHT CLICK → canvas context menu (new terminal, auto-layout)
    │   │   MIDDLE DRAG → pan canvas
    │   │   CTRL+SCROLL → zoom canvas
    │   │
    │   └── Multi-select (Shift+click, or drag-select box on background)
```

### 15.3 Terminal Input Encoding

Map egui key events to the byte sequences that terminals expect:

```rust
fn key_to_bytes(key: egui::Key, modifiers: Modifiers, mode: TerminalMode) -> Vec<u8> {
    match key {
        Key::Enter => vec![b'\r'],
        Key::Backspace => vec![0x7f],
        Key::Tab => vec![b'\t'],
        Key::Escape => vec![0x1b],
        Key::ArrowUp => b"\x1b[A".to_vec(),
        Key::ArrowDown => b"\x1b[B".to_vec(),
        Key::ArrowRight => b"\x1b[C".to_vec(),
        Key::ArrowLeft => b"\x1b[D".to_vec(),
        Key::Home => b"\x1b[H".to_vec(),
        Key::End => b"\x1b[F".to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::Insert => b"\x1b[2~".to_vec(),
        Key::F1 => b"\x1bOP".to_vec(),
        Key::F2 => b"\x1bOQ".to_vec(),
        // ... etc for F3-F12
        // Ctrl+letter: subtract 0x60 from ASCII
        // Alt+letter: prefix with ESC (0x1b)
        _ => {
            if let Some(c) = key_to_char(key) {
                if modifiers.ctrl {
                    vec![(c as u8) - 0x60]
                } else if modifiers.alt {
                    vec![0x1b, c as u8]
                } else {
                    c.to_string().into_bytes()
                }
            } else {
                vec![]
            }
        }
    }
}
```

---

## 16. PTY Management

### 16.1 PTY Lifecycle

```
1. User triggers "New Terminal"
2. Create PanelState with default position + size
3. Open PTY pair: pty_system.openpty(PtySize { rows, cols, ... })
4. Configure environment:
   - TERM=xterm-256color
   - COLORTERM=truecolor
   - VOID_TERMINAL=1 (custom env for detection)
   - Inherit user's PATH, HOME, SHELL, etc.
5. Spawn shell process: pty_pair.slave.spawn_command(cmd)
6. Start reader thread:
   - Continuously read bytes from pty_pair.master.try_clone_reader()
   - Feed bytes to term.lock().perform(Action::Process(bytes))
   - Set dirty flag
   - Request egui repaint
7. Store PanelRuntime in HashMap<PanelId, PanelRuntime>
8. Terminal is now live and rendering
```

### 16.2 PTY Cleanup

```
1. User closes terminal (or process exits)
2. If process exited:
   - Mark panel as "exited" state
   - Panel stays on canvas until user closes it manually
   - Or auto-close after N seconds (configurable)
3. On manual close:
   - Send SIGHUP to child process (Unix) or terminate (Windows)
   - Wait for child process with timeout (2 seconds)
   - If still alive, SIGKILL / force terminate
   - Close PTY master (drops the PTY pair)
   - Join reader thread
   - Remove PanelRuntime from HashMap
   - Remove PanelState from workspace
   - Save state
```

### 16.3 PTY Resize

```
When panel is resized on canvas:
1. Compute new rows/cols from panel pixel size and cell dimensions
   - cols = floor((panel_width - padding) / cell_width)
   - rows = floor((panel_height - title_bar_height - padding) / cell_height)
2. If rows or cols changed from last resize:
   - pty_master.resize(PtySize { rows, cols, pixel_width, pixel_height })
   - term.lock().resize(SizeInfo::new(cols, rows, ...))
   - This sends SIGWINCH to the child process (Unix)
3. Debounce: don't resize more than once per 50ms during continuous drag resize
```

---

## 17. Serialization & Persistence Format

### 17.1 State File (state.json)

```json
{
  "version": 1,
  "last_saved": "2026-03-22T14:30:00Z",
  "active_workspace_id": "uuid-...",
  "workspaces": [
    {
      "id": "uuid-...",
      "name": "Default",
      "color": null,
      "panel_ids": ["uuid-1", "uuid-2", "uuid-3"],
      "viewport": {
        "pan": [0.0, 0.0],
        "zoom": 1.0
      },
      "created_at": 1711100000,
      "last_accessed": 1711100000
    }
  ],
  "panels": {
    "uuid-1": {
      "id": "uuid-1",
      "position": [100.0, 100.0],
      "size": [600.0, 400.0],
      "title": "~/projects/void",
      "shell": "/bin/zsh",
      "cwd": "/home/user/projects/void",
      "color_tag": [0, 212, 255, 255],
      "z_index": 2,
      "created_at": 1711100000
    }
  },
  "sidebar": {
    "visible": true,
    "width": 240.0,
    "collapsed_sections": []
  },
  "window": {
    "position": [100, 100],
    "size": [1920, 1080],
    "maximized": false,
    "fullscreen": false
  }
}
```

### 17.2 Migration Strategy

The `"version"` field enables state file migrations when the schema evolves:

```rust
fn load_state(path: &Path) -> Result<AppState> {
    let raw: serde_json::Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let version = raw["version"].as_u64().unwrap_or(0);
    
    match version {
        0 => migrate_v0_to_v1(raw),
        1 => serde_json::from_value(raw).map_err(Into::into),
        _ => Err(anyhow!("Unknown state version: {version}")),
    }
}
```

---

## 18. Configuration File Format

### 18.1 Complete void.toml Schema

```toml
# Void Terminal Configuration
# Location: ~/.config/void/void.toml (Linux)
#           ~/Library/Application Support/com.void.terminal/void.toml (macOS)
#           %APPDATA%\void\void.toml (Windows)

# --- General ---

[general]
# Default shell to spawn (empty = detect from $SHELL or system default)
shell = ""
# Default working directory for new terminals (empty = home directory)
working_directory = ""
# Close terminal panels automatically when the process exits
auto_close_on_exit = false
# Seconds to wait before auto-closing (if auto_close_on_exit = true)
auto_close_delay = 5
# Confirm before closing a terminal with a running process
confirm_close_running = true

# --- Canvas ---

[canvas]
# Show dot grid background
grid_visible = false
# Grid spacing in pixels (at 100% zoom)
grid_spacing = 50
# Snap panels to grid when dragging
snap_to_grid = false
# Zoom range
zoom_min = 0.1
zoom_max = 4.0
# Default zoom for new workspaces
zoom_default = 1.0
# Pan/zoom animation duration in milliseconds
animation_duration_ms = 200
# Pan inertia (0.0 = no inertia, 1.0 = maximum inertia)
pan_inertia = 0.85

# --- Panels ---

[panels]
# Default panel size in pixels (at 100% zoom)
default_width = 600
default_height = 400
# Minimum panel size in columns/rows
min_cols = 40
min_rows = 10
# Panel border radius in pixels
border_radius = 4.0
# Panel border width in pixels
border_width = 1.0
# Spacing between panels for auto-layout (pixels)
layout_gap = 20
# Show panel shadows
shadows = true
# Shadow blur radius
shadow_blur = 8.0
# Shadow color (RGBA hex)
shadow_color = "#00000040"

# --- Terminal ---

[terminal]
# Scrollback buffer size (lines)
scrollback_lines = 10000
# Copy on select (like X11 primary selection)
copy_on_select = false
# Cursor style: "block", "beam", "underline"
cursor_style = "block"
# Cursor blinking
cursor_blink = true
# Cursor blink interval in milliseconds
cursor_blink_ms = 500
# Bell behavior: "none", "visual", "sound"
bell = "visual"
# Word separators for double-click word selection
word_separators = " \t{}[]()\"'`,;:@=|\\/"
# Enable bracketed paste mode
bracketed_paste = true
# Mouse reporting (let terminal apps capture mouse)
mouse_reporting = true
# URL detection and click-to-open
url_detection = true
# URL modifier key (hold this + click to open URL)
url_modifier = "Ctrl"

# --- Font ---

[font]
# Font family (must be a monospace font)
family = "JetBrains Mono"
# Font size in points
size = 13.0
# Line height multiplier
line_height = 1.2
# Enable ligatures
ligatures = false
# Bold text uses bright colors (terminal convention)
bold_is_bright = false
# Use the OS default monospace font if the specified font is not found
fallback_to_system = true

# --- Sidebar ---

[sidebar]
# Show sidebar on startup
visible = true
# Default width in pixels
width = 240
# Position: "left" or "right"
position = "left"

# --- Minimap ---

[minimap]
# Show minimap on startup
visible = true
# Minimap size in pixels
width = 200
height = 150
# Corner position: "top-left", "top-right", "bottom-left", "bottom-right"
position = "bottom-right"
# Opacity (0.0 - 1.0)
opacity = 0.8

# --- Theme ---

[theme]
# Built-in theme: "void-dark", "void-light", "catppuccin-mocha", "dracula", "nord"
# Or path to custom theme file
name = "void-dark"

# --- Keybindings ---

[keybindings]
new_terminal = "Ctrl+Shift+T"
close_terminal = "Ctrl+Shift+W"
command_palette = "Ctrl+Shift+P"
toggle_sidebar = "Ctrl+B"
toggle_minimap = "Ctrl+M"
toggle_fullscreen = "F11"
zoom_in = "Ctrl+="
zoom_out = "Ctrl+-"
zoom_reset = "Ctrl+0"
zoom_fit_all = "Ctrl+Shift+0"
next_terminal = "Ctrl+Shift+]"
prev_terminal = "Ctrl+Shift+["
workspace_1 = "Ctrl+1"
workspace_2 = "Ctrl+2"
workspace_3 = "Ctrl+3"
workspace_4 = "Ctrl+4"
workspace_5 = "Ctrl+5"
workspace_6 = "Ctrl+6"
workspace_7 = "Ctrl+7"
workspace_8 = "Ctrl+8"
workspace_9 = "Ctrl+9"
rename_terminal = "F2"
copy = "Ctrl+Shift+C"
paste = "Ctrl+Shift+V"
select_all = "Ctrl+Shift+A"
clear_terminal = "Ctrl+Shift+K"
find_in_terminal = "Ctrl+Shift+F"
toggle_fps = "Ctrl+Shift+F12"

# --- Advanced ---

[advanced]
# GPU backend preference: "auto", "vulkan", "metal", "dx12", "opengl"
gpu_backend = "auto"
# Maximum frame rate (0 = vsync/unlimited)
max_fps = 0
# Environment variables to set for all terminals
[advanced.env]
# EDITOR = "vim"

# --- Logging ---

[logging]
# Log level: "error", "warn", "info", "debug", "trace"
level = "warn"
# Log to file (in data directory)
file = false
```

---

## 19. Build System & CI/CD

### 19.1 Local Development

```bash
# Prerequisites:
# - Rust toolchain (rustup.rs) — stable, latest
# - System deps for wgpu:
#   Linux: libxkbcommon-dev libwayland-dev vulkan-tools (or mesa)
#   macOS: Xcode command line tools
#   Windows: Visual Studio Build Tools

# Clone
git clone https://github.com/<user>/void.git
cd void

# Run Void
cargo run

# Build release
cargo build --release

# Lint
cargo clippy -- -D warnings
cargo fmt --check

# Test
cargo test
```

### 19.2 CI Pipeline (.github/workflows/ci.yml)

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings

  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Linux deps
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev
      - run: cargo test

  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Install Linux deps
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev
      - run: cargo build --release --target ${{ matrix.target }}
```

### 19.3 Release Pipeline (cargo-dist)

We use **cargo-dist** (by axodotdev) to automatically build and publish binaries for all platforms when a version tag is pushed.

**dist.toml (at repo root):**

```toml
[dist]
cargo-dist-version = "0.31.0"
ci = "github"
installers = ["shell", "powershell", "msi"]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
install-path = "CARGO_HOME"
```

**Release process:**

```bash
# Bump version in Cargo.toml
# Commit
git tag v0.1.0
git push --tags
# CI builds all artifacts and creates GitHub Release
```

---

## 20. Distribution & Installation

### 20.1 Installation Methods (by Platform)

**Linux:**
```bash
# Shell installer (recommended)
curl -LsSf https://github.com/<user>/void/releases/latest/download/void-installer.sh | sh

# Or download .deb
sudo dpkg -i void_0.1.0_amd64.deb

# Or download .tar.gz and extract
tar xzf void-x86_64-unknown-linux-gnu.tar.gz
./void

# Or cargo install (builds from source)
cargo install void-terminal
```

**macOS:**
```bash
# Shell installer (recommended)
curl -LsSf https://github.com/<user>/void/releases/latest/download/void-installer.sh | sh

# Or download .dmg and drag to Applications

# Or Homebrew (future — need to set up a tap)
brew install <user>/tap/void
```

**Windows:**
```powershell
# PowerShell installer (recommended)
irm https://github.com/<user>/void/releases/latest/download/void-installer.ps1 | iex

# Or download .msi and run installer

# Or download .exe (portable, no install needed)

# Or winget (future)
winget install void-terminal
```

### 20.2 Future Package Managers

- **Homebrew** (macOS/Linux) — via tap
- **Winget** (Windows) — submit manifest
- **Scoop** (Windows) — bucket
- **AUR** (Arch Linux) — PKGBUILD
- **Flathub** (Linux) — Flatpak manifest
- **Nix** (NixOS/Nix) — flake

---

## 21. Documentation Site (Fumadocs)

The documentation site lives at `apps/site/` and uses Fumadocs (Next.js-based).

### 21.1 Documentation Structure

```
content/docs/
├── index.mdx                    # Welcome / overview
├── getting-started.mdx          # Quick start guide
├── installation/
│   ├── index.mdx                # Overview of install methods
│   ├── linux.mdx
│   ├── macos.mdx
│   └── windows.mdx
├── guides/
│   ├── index.mdx
│   ├── first-terminal.mdx       # Spawn your first terminal
│   ├── canvas-navigation.mdx    # Pan, zoom, minimap
│   ├── workspaces.mdx           # Creating and using workspaces
│   ├── auto-layout.mdx          # Layout modes
│   └── customization.mdx        # Theming, fonts, colors
├── reference/
│   ├── index.mdx
│   ├── configuration.mdx        # Full void.toml reference
│   ├── keyboard-shortcuts.mdx   # Complete shortcut table
│   ├── cli.mdx                  # CLI arguments reference
│   └── environment.mdx          # Environment variables
├── development/
│   ├── index.mdx
│   ├── architecture.mdx         # Codebase architecture
│   ├── building.mdx             # Building from source
│   └── contributing.mdx         # Contribution guide
└── changelog.mdx                # Versioned changelog
```

### 21.2 Deployment

- **Vercel** (recommended) — automatic deploy on push to `main`
- Custom domain: `docs.void.sh` or `void.sh/docs`
- Preview deployments for PRs

---

## 22. Landing Page / Website

The landing page is the `/` route of the Fumadocs Next.js app.

### 22.1 Page Structure

```
[Navbar: Logo | Docs | GitHub | Download]

[Hero Section]
  "Void"
  "Where your terminals float free."
  [Download Button] [View on GitHub]
  [Hero screenshot/GIF showing canvas with multiple terminals]

[Features Grid]
  - Infinite Canvas: Pan, zoom, place terminals anywhere
  - GPU Accelerated: 60fps with 30+ terminals (wgpu)
  - Cross-Platform: Windows, Linux, macOS
  - Persistent Sessions: Your layout survives restarts
  - Keyboard-First: Every action without touching the mouse
  - Open Source: MIT license, community-driven

[Demo Video/GIF Section]
  Animated recording showing: spawn terminals, pan canvas,
  zoom out to overview, auto-layout, switch workspaces

[Download Section]
  Platform-detected download button
  Links for all platforms

[Footer: GitHub | Docs | License | Made with Rust]
```

### 22.2 Design Direction

- Dark aesthetic matching Void's dark theme
- Gradient from deep black to subtle dark blue
- Monospace headings (matching the terminal aesthetic)
- Minimalist, fast-loading, no unnecessary JavaScript

---

## 23. Development Phases & Milestones

### Phase 0: Scaffolding (Week 1)

- [ ] Create monorepo structure (Turborepo + pnpm)
- [ ] Initialize Rust project with eframe + egui
- [ ] Setup CI (lint + test + build matrix)
- [ ] Create Fumadocs site skeleton
- [ ] Choose and bundle default monospace font
- [ ] Basic eframe window opens with egui, black background
- [ ] README with project description + status

**Deliverable:** Empty window opens on all 3 platforms. CI passes. Docs site deploys.

### Phase 1: Canvas Foundation (Week 2-3)

- [ ] Implement egui::Scene integration (pan + zoom)
- [ ] Render colored rectangles as placeholder "panels" on the canvas
- [ ] Draggable panels (click title bar, move on canvas)
- [ ] Resizable panels (drag edges/corners)
- [ ] Panel z-order (click to bring to front)
- [ ] Viewport culling (only render visible panels)
- [ ] Basic minimap (shows panel positions as colored dots)
- [ ] Canvas background (optional grid)

**Deliverable:** You can pan/zoom an infinite canvas with draggable, resizable colored rectangles.

### Phase 2: Terminal Integration (Week 4-6)

- [ ] Integrate `portable-pty` — spawn a shell process
- [ ] Integrate `alacritty_terminal` — create Term instance, feed PTY output
- [ ] Implement terminal renderer (grid cells → egui painter)
- [ ] Keyboard input → PTY (basic keys: letters, Enter, Backspace, arrows)
- [ ] Full keyboard input (Ctrl+C, Ctrl+D, function keys, Alt+...)
- [ ] PTY resize on panel resize
- [ ] Multiple simultaneous terminals (each with own PTY + thread)
- [ ] Terminal cursor rendering (block, beam, underline, blinking)
- [ ] Color support (ANSI 16 + 256 + truecolor)
- [ ] Scrollback buffer (scroll with mouse wheel on focused panel)

**Deliverable:** Fully functional terminal panels on the infinite canvas. You can run shells, vim, htop, etc.

### Phase 3: UI Polish (Week 7-9)

- [ ] Left sidebar implementation
  - [ ] Workspace list (create, switch, rename, delete)
  - [ ] Terminal session list (click to focus, show status)
  - [ ] Quick actions (new terminal, settings link)
  - [ ] Sidebar collapse/expand with animation
- [ ] Command palette
  - [ ] Fuzzy matching
  - [ ] Command registry
  - [ ] Keyboard shortcut hints
- [ ] Panel title bar (CWD + shell name, close button, rename)
- [ ] Panel context menu (right-click)
- [ ] Panel visual states (focused, unfocused, exited, error)
- [ ] Panel color tags
- [ ] Toast notifications
- [ ] Workspace switching with animated viewport transition
- [ ] Focus cycling: Ctrl+Shift+] and Ctrl+Shift+[

**Deliverable:** Complete UI with sidebar, command palette, workspaces. Feels like a real application.

### Phase 4: Persistence & Config (Week 10-11)

- [ ] State serialization (save to JSON)
- [ ] State deserialization (load on startup, restore all panels + workspaces)
- [ ] Debounced autosave
- [ ] Configuration file (void.toml) loading
- [ ] Config hot-reload (file watcher)
- [ ] All config options wired up
- [ ] Window geometry save/restore
- [ ] Settings UI (either in sidebar or a dedicated panel/modal)

**Deliverable:** Close Void, reopen it, everything is exactly where you left it. Configuration works.

### Phase 5: Theming & Fonts (Week 12)

- [ ] Theme system (load from config, apply to egui)
- [ ] Built-in themes: void-dark, void-light, catppuccin-mocha, dracula, nord
- [ ] Custom theme file support
- [ ] Font configuration (family, size, line height)
- [ ] Font fallback chain
- [ ] Bold/italic font variant loading
- [ ] Correct wide character (CJK) rendering

**Deliverable:** Beautiful, customizable appearance. Multiple built-in themes.

### Phase 6: Auto-Layout & Advanced Canvas (Week 13)

- [ ] Auto-layout algorithms (grid, rows, columns, stack, cascade)
- [ ] Animated layout transitions
- [ ] Grid snapping (optional)
- [ ] Multi-select panels (Shift+click, drag-select box)
- [ ] Group operations (move group, close group, assign group to workspace)
- [ ] Minimap enhancements (click-to-navigate, drag viewport, zoom indicator)
- [ ] Simplified panel view at extreme zoom-out

**Deliverable:** Advanced canvas features. Auto-layout. Multi-select. Full minimap.

### Phase 7: Terminal Advanced Features (Week 14-15)

- [ ] Text selection (click-drag, double-click word, triple-click line)
- [ ] Copy/paste (Ctrl+Shift+C/V, copy-on-select option)
- [ ] URL detection (highlight, Ctrl+click to open)
- [ ] Mouse reporting (pass mouse events to terminal apps)
- [ ] Bracketed paste mode
- [ ] Alternate screen buffer (correct behavior for vim, htop, etc.)
- [ ] Terminal bell (visual flash or system sound)
- [ ] Find in terminal (search scrollback buffer)
- [ ] Clear scrollback command

**Deliverable:** Terminal is feature-complete. All standard terminal operations work.

### Phase 8: Distribution & Packaging (Week 16)

- [ ] cargo-dist setup
- [ ] Release CI workflow
- [ ] Test installation on all platforms
- [ ] DMG build for macOS
- [ ] MSI build for Windows
- [ ] AppImage or .deb for Linux
- [ ] Shell/PowerShell installer scripts
- [ ] README install instructions
- [ ] `cargo install` support (publish to crates.io)

**Deliverable:** Users can install Void with one command on any platform.

### Phase 9: Documentation & Website (Week 17)

- [ ] Complete all docs pages (see Section 21)
- [ ] Landing page design + implementation
- [ ] Demo GIF/video creation
- [ ] Deploy to Vercel
- [ ] Custom domain setup
- [ ] OpenGraph/social media preview images

**Deliverable:** Professional website + comprehensive docs.

### Phase 10: Beta Release (Week 18)

- [ ] Final testing on all platforms
- [ ] Performance profiling and optimization pass
- [ ] Bug sweep
- [ ] CONTRIBUTING.md
- [ ] LICENSE
- [ ] Tag v0.1.0-beta.1
- [ ] Announce on Reddit (r/rust, r/commandline), Hacker News, Twitter/X

**Deliverable:** Public beta release. Open for community feedback and contributions.

---

## 24. Testing Strategy

### 24.1 Unit Tests

- **Config parsing** — test TOML deserialization, default values, invalid configs
- **State serialization** — round-trip: AppState → JSON → AppState
- **Fuzzy matching** — test the command palette fuzzy scorer
- **Terminal input encoding** — test key_to_bytes for all key combinations
- **Color mapping** — test alacritty Color → egui Color32 conversion
- **Viewport math** — test screen ↔ canvas coordinate transforms
- **Layout algorithms** — test auto-layout produces correct positions

### 24.2 Integration Tests

- **PTY spawn + read/write** — spawn a shell, send commands, verify output
- **Terminal emulation** — send VT escape sequences, verify grid state
- **Config hot-reload** — modify config file, verify changes apply
- **State persistence** — save state, modify, load, verify match

### 24.3 Manual Testing Checklist

For each release:

- [ ] Fresh install on Linux (Ubuntu 24.04)
- [ ] Fresh install on macOS (latest)
- [ ] Fresh install on Windows 11
- [ ] Spawn 30 terminals, verify 60fps
- [ ] Run vim in a terminal, verify correct rendering
- [ ] Run htop, verify alternate screen buffer
- [ ] Resize a terminal while running a TUI app
- [ ] Copy/paste text between terminals
- [ ] Switch between workspaces
- [ ] Close and reopen Void, verify state restoration
- [ ] Modify config file while Void is running
- [ ] Test all keyboard shortcuts

---

## 25. Accessibility

### 25.1 Goals

- High contrast default theme
- All actions reachable via keyboard
- Configurable font sizes (no minimum enforced)
- No reliance on color alone for conveying information (always include text labels)

### 25.2 Future Considerations (Post-v1)

- Screen reader support (egui has nascent AccessKit integration)
- Announce terminal output to screen reader
- Reduced motion mode (disable animations)

---

## 26. Security Considerations

### 26.1 PTY Security

- Terminals inherit the user's permissions — Void does not run with elevated privileges
- Shell processes are spawned with the user's default shell and environment
- No remote execution capability (no SSH integration built-in — users use their shell's ssh)

### 26.2 Config File Security

- Config file is user-owned with standard file permissions
- No sensitive data in config (no passwords, tokens, etc.)
- State file may contain terminal scrollback — store in user data directory with appropriate permissions

### 26.3 Clipboard

- Clipboard access is explicit (user-initiated copy/paste)
- No background clipboard monitoring
- Bracketed paste protects against paste injection attacks

---

## 27. Future / Post-v1 Features

These are explicitly **out of scope** for v1 but considered for future development:

1. **Plugin system** — Lua or WASM-based plugins for custom behavior
2. **AI integration** — optional AI assistant panel (like Horizon's feature)
3. **SSH integration** — built-in SSH session management
4. **Split view** — split a panel into sub-panels (tmux-style, but within a canvas panel)
5. **Markdown viewer** — drop a .md file on canvas, render as formatted panel
6. **Image protocol** — iTerm2/Kitty image protocol support (inline images in terminal)
7. **SIXEL support** — render SIXEL graphics in terminal
8. **Tab completion** — custom autocomplete layer over shell
9. **Session sharing** — share a terminal session with another user (collaborative)
10. **Recording/playback** — record terminal sessions as asciinema files
11. **Remote sync** — sync workspaces/layouts across machines
12. **Vim/modal mode** — canvas navigation using vim-style keys (hjkl)
13. **Scripting** — automate Void actions via CLI or script
14. **Tray icon** — minimize to system tray, quick-launch terminals
15. **Drag-and-drop** — drop files onto a terminal to paste their path

---

## 28. Open Questions & Decisions

| # | Question | Options | Recommendation | Status |
|---|----------|---------|---------------|--------|
| 1 | Crate name on crates.io | `void`, `void-terminal`, `void-term`, `void-canvas` | `void-terminal` (void is likely taken) | Open |
| 2 | Bundle font or use system fonts? | Bundle JetBrains Mono / Use system monospace | Bundle (ensures consistent rendering) | Bundle |
| 3 | Scrollback persistence | Save full scrollback / Save last N lines / Don't save | Save last 1000 lines (configurable) | Open |
| 4 | Terminal scrollback format | Store as raw text / Store as VT stream / Store as JSON cells | Raw text (simplest, smallest) | Open |
| 5 | macOS signing | Apple Developer account ($99/yr) or unsigned | Unsigned for v1, signed later | Open |
| 6 | License | MIT / Apache-2.0 / MIT + Apache-2.0 dual | MIT (simplest, most permissive) | MIT |
| 7 | egui::Scene availability | Scene was added recently — verify latest egui includes it | Check egui 0.30+ changelog | Verify |
| 8 | Font rendering approach | egui built-in text / custom glyph atlas with wgpu | egui built-in for v1, optimize later if needed | egui built-in |
| 9 | Default panel spawn position | Center of viewport / Offset from last panel / Random | Center of viewport with smart offset (avoid overlap) | Center + offset |
| 10 | Config file format | TOML / YAML / JSON | TOML (Rust standard, clean syntax) | TOML |

---

## Appendix A: Cargo.toml Templates

### Cargo.toml

```toml
[package]
name = "void-terminal"
version = "0.1.0"
edition = "2021"
description = "Infinite canvas terminal emulator — GPU-accelerated, cross-platform"
license = "MIT"
repository = "https://github.com/<user>/void"
homepage = "https://void.sh"
readme = "README.md"
keywords = ["terminal", "gpu", "canvas", "egui", "wgpu"]
categories = ["command-line-utilities", "gui"]

[[bin]]
name = "void"
path = "src/main.rs"

[dependencies]
# GUI
eframe = { version = "0.30", default-features = false, features = ["wgpu", "persistence"] }
egui = "0.30"

# Terminal emulation
alacritty_terminal = "0.24"
portable-pty = "0.9"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Utilities
anyhow = "1"
log = "0.4"
env_logger = "0.11"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
directories = "5"
notify = "7"
arboard = "3"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.dev]
opt-level = 1  # Slightly optimize dev builds for usable performance

[package.metadata.dist]
installers = ["shell", "powershell", "msi"]
```

---

## Appendix B: Key Data Structures

### Complete AppState (Serialized)

```rust
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub version: u32,
    pub last_saved: String,  // ISO 8601
    pub active_workspace_id: Uuid,
    pub workspaces: Vec<Workspace>,
    pub panels: HashMap<Uuid, PanelState>,
    pub focused_panel_id: Option<Uuid>,
    pub sidebar: SidebarState,
    pub window: WindowState,
}

#[derive(Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub color: Option<[u8; 4]>,
    pub panel_ids: Vec<Uuid>,
    pub viewport: ViewportState,
    pub created_at: i64,
    pub last_accessed: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ViewportState {
    pub pan: [f32; 2],
    pub zoom: f32,
}

#[derive(Serialize, Deserialize)]
pub struct PanelState {
    pub id: Uuid,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub title: String,
    pub shell: String,
    pub cwd: String,
    pub color_tag: Option<[u8; 4]>,
    pub z_index: u32,
    pub created_at: i64,
    pub scrollback_snapshot: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SidebarState {
    pub visible: bool,
    pub width: f32,
    pub collapsed_sections: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct WindowState {
    pub position: Option<[i32; 2]>,
    pub size: [u32; 2],
    pub maximized: bool,
    pub fullscreen: bool,
}
```

---

## Appendix C: Keyboard Shortcuts Reference

### Global Shortcuts (Work Everywhere)

| Action | Shortcut | macOS |
|--------|----------|-------|
| New Terminal | `Ctrl+Shift+T` | `Cmd+Shift+T` |
| Close Terminal | `Ctrl+Shift+W` | `Cmd+Shift+W` |
| Command Palette | `Ctrl+Shift+P` | `Cmd+Shift+P` |
| Toggle Sidebar | `Ctrl+B` | `Cmd+B` |
| Toggle Minimap | `Ctrl+M` | `Cmd+M` |
| Toggle Fullscreen | `F11` | `Cmd+Ctrl+F` |
| Zoom In | `Ctrl+=` | `Cmd+=` |
| Zoom Out | `Ctrl+-` | `Cmd+-` |
| Zoom Reset (100%) | `Ctrl+0` | `Cmd+0` |
| Zoom to Fit All | `Ctrl+Shift+0` | `Cmd+Shift+0` |
| Next Terminal | `Ctrl+Shift+]` | `Cmd+Shift+]` |
| Previous Terminal | `Ctrl+Shift+[` | `Cmd+Shift+[` |
| Switch to Workspace 1-9 | `Ctrl+1` through `Ctrl+9` | `Cmd+1` through `Cmd+9` |
| Rename Terminal | `F2` | `F2` |
| Copy | `Ctrl+Shift+C` | `Cmd+C` |
| Paste | `Ctrl+Shift+V` | `Cmd+V` |
| Select All | `Ctrl+Shift+A` | `Cmd+A` |
| Clear Terminal | `Ctrl+Shift+K` | `Cmd+K` |
| Find in Terminal | `Ctrl+Shift+F` | `Cmd+Shift+F` |
| Toggle FPS Counter | `Ctrl+Shift+F12` | `Cmd+Shift+F12` |
| Settings | `Ctrl+,` | `Cmd+,` |
| Quit | `Ctrl+Q` | `Cmd+Q` |

### Canvas Shortcuts (When No Terminal Focused)

| Action | Shortcut |
|--------|----------|
| Pan (hold) | `Space` + drag |
| Pan Up/Down/Left/Right | Arrow keys |
| Select All Panels | `Ctrl+A` |
| Delete Selected Panels | `Delete` / `Backspace` |
| Auto-Layout: Grid | `Ctrl+Shift+G` |

### Terminal Shortcuts (When Terminal Focused)

All standard terminal key sequences pass through (Ctrl+C, Ctrl+D, Ctrl+Z, etc.). Void shortcuts use `Ctrl+Shift+` prefix to avoid conflicts.

---

## Appendix D: ANSI/VT Escape Sequences to Support

### Handled by alacritty_terminal (included, no custom work needed)

- **C0 controls:** BEL, BS, HT, LF, VT, FF, CR, SO, SI, ESC
- **C1 controls:** CSI, OSC, DCS, PM, APC, SS2, SS3
- **CSI sequences:** CUU, CUD, CUF, CUB, CHA, CUP, ED, EL, IL, DL, DCH, ICH, SU, SD, SGR, DECSTBM, DECSET, DECRST, and more
- **SGR attributes:** Bold, dim, italic, underline, blink, inverse, invisible, strikethrough, 256-color, truecolor
- **OSC sequences:** Set title (0, 1, 2), hyperlinks (8), clipboard (52)
- **DCS sequences:** tmux passthrough, DECRQSS
- **Mouse modes:** 1000, 1002, 1003, 1006 (SGR extended)
- **Misc:** Bracketed paste (2004), Focus events (1004), Alternate screen buffer (1049)

### Not Supported in v1 (Future)

- SIXEL graphics
- iTerm2 image protocol
- Kitty image protocol

---

## Appendix E: Competitor Feature Matrix

| Feature | Void (planned) | Horizon | Finite | Alacritty | WezTerm | Kitty |
|---------|:-:|:-:|:-:|:-:|:-:|:-:|
| Infinite canvas | Y | Y | Y | N | N | N |
| Pan & zoom | Y | Y | Y | N | N | N |
| Minimap | Y | Y | Y | N | N | N |
| GPU accelerated | Y | Y | Y | Y | Y | Y |
| Cross-platform | Y | Y | N (macOS) | Y | Y | Y |
| 100% Rust | Y | Y | N (Swift) | Y | Y | N (C/Py) |
| Session persistence | Y | Y | Y | N | Partial | N |
| Workspaces | Y | Y | ? | N | N | N |
| Left sidebar | Y | N | ? | N | N | N |
| Command palette | Y | Y | ? | N | Y | N |
| Auto-layout | Y | Y | ? | N | N | N |
| Theming | Y | Y | ? | Y | Y | Y |
| Config hot-reload | Y | Y | ? | Y | Y | Y |
| Splits/tabs | N (by design) | N | N | N | Y | Y |
| Open source | Y (MIT) | Y (MIT) | N | Y (Apache) | Y (MIT) | Y (GPL) |
| Docs site | Y (Fumadocs) | Partial | ? | Y | Y | Y |
| Multi-platform installer | Y (cargo-dist) | Y | N | Y | Y | Y |

---

*End of PRD. This document should be treated as a living specification — update it as decisions are made and implementations diverge from initial plans.*