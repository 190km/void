// Command registry — all available actions in the command palette

/// Actions that can be triggered from the command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    NewTerminal,
    CloseTerminal,
    RenameTerminal,
    ToggleSidebar,
    ToggleMinimap,
    ToggleGrid,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomToFit,
    FocusNext,
    FocusPrev,
    ToggleFullscreen,
}

/// A registered command with display info.
pub struct CommandEntry {
    pub command: Command,
    pub label: &'static str,
    pub shortcut: &'static str,
}

/// All registered commands.
pub const COMMANDS: &[CommandEntry] = &[
    CommandEntry {
        command: Command::NewTerminal,
        label: "New Terminal",
        shortcut: "Ctrl+Shift+T",
},
    CommandEntry {
        command: Command::CloseTerminal,
        label: "Close Terminal",
        shortcut: "Ctrl+Shift+W",
},
    CommandEntry {
        command: Command::RenameTerminal,
        label: "Rename Terminal",
        shortcut: "F2",
},
    CommandEntry {
        command: Command::FocusNext,
        label: "Focus Next Terminal",
        shortcut: "Ctrl+Shift+]",
},
    CommandEntry {
        command: Command::FocusPrev,
        label: "Focus Previous Terminal",
        shortcut: "Ctrl+Shift+[",
},
    CommandEntry {
        command: Command::ZoomToFit,
        label: "Zoom to Fit All",
        shortcut: "Ctrl+Shift+0",
},
    CommandEntry {
        command: Command::ToggleSidebar,
        label: "Toggle Sidebar",
        shortcut: "Ctrl+B",
},
    CommandEntry {
        command: Command::ToggleMinimap,
        label: "Toggle Minimap",
        shortcut: "Ctrl+M",
},
    CommandEntry {
        command: Command::ToggleGrid,
        label: "Toggle Grid",
        shortcut: "Ctrl+G",
},
    CommandEntry {
        command: Command::ZoomIn,
        label: "Zoom In",
        shortcut: "Ctrl+=",
},
    CommandEntry {
        command: Command::ZoomOut,
        label: "Zoom Out",
        shortcut: "Ctrl+-",
},
    CommandEntry {
        command: Command::ZoomReset,
        label: "Reset Zoom",
        shortcut: "Ctrl+0",
},
    CommandEntry {
        command: Command::ToggleFullscreen,
        label: "Toggle Fullscreen",
        shortcut: "F11",
},
];
