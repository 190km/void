// Maps egui keyboard events to terminal input bytes

use egui::{Event, Key, Modifiers};

/// Process egui input events and return bytes to write to the PTY.
/// Returns None if no terminal input was generated.
pub fn process_input(ctx: &egui::Context, void_shortcuts: &[(Modifiers, Key)]) -> Vec<u8> {
    let mut output = Vec::new();

    ctx.input(|input| {
        for event in &input.events {
            match event {
                Event::Text(text) => {
                    // Regular text input (letters, numbers, punctuation)
                    // Only if no ctrl/alt modifier (those are handled via Key events)
                    if !input.modifiers.ctrl && !input.modifiers.alt {
                        output.extend_from_slice(text.as_bytes());
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    // Skip Void app shortcuts
                    if is_void_shortcut(modifiers, key, void_shortcuts) {
                        continue;
                    }

                    if let Some(bytes) = key_to_bytes(key, modifiers) {
                        output.extend_from_slice(&bytes);
                    }
                }
                Event::Paste(text) => {
                    // Bracketed paste mode support
                    output.extend_from_slice(b"\x1b[200~");
                    output.extend_from_slice(text.as_bytes());
                    output.extend_from_slice(b"\x1b[201~");
                }
                _ => {}
            }
        }
    });

    output
}

/// Check if a key combo is a Void app shortcut that should not be sent to the terminal.
fn is_void_shortcut(modifiers: &Modifiers, key: &Key, shortcuts: &[(Modifiers, Key)]) -> bool {
    shortcuts
        .iter()
        .any(|(m, k)| m.ctrl == modifiers.ctrl && m.shift == modifiers.shift && k == key)
}

/// Convert a key press + modifiers to terminal byte sequence.
fn key_to_bytes(key: &Key, modifiers: &Modifiers) -> Option<Vec<u8>> {
    // Ctrl+letter → control character
    if modifiers.ctrl && !modifiers.shift && !modifiers.alt {
        if let Some(byte) = ctrl_key_byte(key) {
            return Some(vec![byte]);
        }
    }

    // Alt+key → ESC prefix
    if modifiers.alt && !modifiers.ctrl {
        if let Some(c) = key_to_char(key) {
            let mut bytes = vec![0x1b]; // ESC
            bytes.extend_from_slice(c.to_string().as_bytes());
            return Some(bytes);
        }
    }

    // Special keys
    match key {
        Key::Enter => Some(b"\r".to_vec()),
        Key::Backspace => Some(b"\x7f".to_vec()),
        Key::Tab => {
            if modifiers.shift {
                Some(b"\x1b[Z".to_vec()) // Shift+Tab = reverse tab
            } else {
                Some(b"\t".to_vec())
            }
        }
        Key::Escape => Some(b"\x1b".to_vec()),
        Key::ArrowUp => Some(csi_modifier(b"A", modifiers)),
        Key::ArrowDown => Some(csi_modifier(b"B", modifiers)),
        Key::ArrowRight => Some(csi_modifier(b"C", modifiers)),
        Key::ArrowLeft => Some(csi_modifier(b"D", modifiers)),
        Key::Home => Some(csi_modifier(b"H", modifiers)),
        Key::End => Some(csi_modifier(b"F", modifiers)),
        Key::PageUp => Some(b"\x1b[5~".to_vec()),
        Key::PageDown => Some(b"\x1b[6~".to_vec()),
        Key::Insert => Some(b"\x1b[2~".to_vec()),
        Key::Delete => Some(b"\x1b[3~".to_vec()),
        Key::F1 => Some(b"\x1bOP".to_vec()),
        Key::F2 => Some(b"\x1bOQ".to_vec()),
        Key::F3 => Some(b"\x1bOR".to_vec()),
        Key::F4 => Some(b"\x1bOS".to_vec()),
        Key::F5 => Some(b"\x1b[15~".to_vec()),
        Key::F6 => Some(b"\x1b[17~".to_vec()),
        Key::F7 => Some(b"\x1b[18~".to_vec()),
        Key::F8 => Some(b"\x1b[19~".to_vec()),
        Key::F9 => Some(b"\x1b[20~".to_vec()),
        Key::F10 => Some(b"\x1b[21~".to_vec()),
        Key::F11 => Some(b"\x1b[23~".to_vec()),
        Key::F12 => Some(b"\x1b[24~".to_vec()),
        Key::Space => {
            if modifiers.ctrl {
                Some(vec![0x00]) // Ctrl+Space = NUL
            } else {
                None // handled by Event::Text
            }
        }
        _ => None, // Handled by Event::Text for regular characters
    }
}

/// Generate CSI sequence with modifier encoding for arrow/nav keys.
fn csi_modifier(suffix: &[u8], modifiers: &Modifiers) -> Vec<u8> {
    let modifier_code = modifier_param(modifiers);
    if modifier_code > 1 {
        // CSI 1 ; <modifier> <suffix>
        let mut seq = b"\x1b[1;".to_vec();
        seq.extend_from_slice(modifier_code.to_string().as_bytes());
        seq.extend_from_slice(suffix);
        seq
    } else {
        let mut seq = b"\x1b[".to_vec();
        seq.extend_from_slice(suffix);
        seq
    }
}

/// xterm modifier parameter: 1 + (shift?1:0) + (alt?2:0) + (ctrl?4:0)
fn modifier_param(modifiers: &Modifiers) -> u8 {
    let mut code: u8 = 1;
    if modifiers.shift {
        code += 1;
    }
    if modifiers.alt {
        code += 2;
    }
    if modifiers.ctrl {
        code += 4;
    }
    code
}

/// Map Ctrl+key to control character byte (0x01–0x1A for A–Z).
fn ctrl_key_byte(key: &Key) -> Option<u8> {
    match key {
        Key::A => Some(0x01),
        Key::B => Some(0x02),
        Key::C => Some(0x03),
        Key::D => Some(0x04),
        Key::E => Some(0x05),
        Key::F => Some(0x06),
        Key::G => Some(0x07),
        Key::H => Some(0x08),
        Key::I => Some(0x09),
        Key::J => Some(0x0A),
        Key::K => Some(0x0B),
        Key::L => Some(0x0C),
        Key::M => Some(0x0D),
        Key::N => Some(0x0E),
        Key::O => Some(0x0F),
        Key::P => Some(0x10),
        Key::Q => Some(0x11),
        Key::R => Some(0x12),
        Key::S => Some(0x13),
        Key::T => Some(0x14),
        Key::U => Some(0x15),
        Key::V => Some(0x16),
        Key::W => Some(0x17),
        Key::X => Some(0x18),
        Key::Y => Some(0x19),
        Key::Z => Some(0x1A),
        Key::OpenBracket => Some(0x1B),  // Ctrl+[ = ESC
        Key::Backslash => Some(0x1C),    // Ctrl+\ = FS
        Key::CloseBracket => Some(0x1D), // Ctrl+] = GS
        _ => None,
    }
}

/// Map egui Key to its character representation (for Alt+key sequences).
fn key_to_char(key: &Key) -> Option<char> {
    match key {
        Key::A => Some('a'),
        Key::B => Some('b'),
        Key::C => Some('c'),
        Key::D => Some('d'),
        Key::E => Some('e'),
        Key::F => Some('f'),
        Key::G => Some('g'),
        Key::H => Some('h'),
        Key::I => Some('i'),
        Key::J => Some('j'),
        Key::K => Some('k'),
        Key::L => Some('l'),
        Key::M => Some('m'),
        Key::N => Some('n'),
        Key::O => Some('o'),
        Key::P => Some('p'),
        Key::Q => Some('q'),
        Key::R => Some('r'),
        Key::S => Some('s'),
        Key::T => Some('t'),
        Key::U => Some('u'),
        Key::V => Some('v'),
        Key::W => Some('w'),
        Key::X => Some('x'),
        Key::Y => Some('y'),
        Key::Z => Some('z'),
        Key::Num0 => Some('0'),
        Key::Num1 => Some('1'),
        Key::Num2 => Some('2'),
        Key::Num3 => Some('3'),
        Key::Num4 => Some('4'),
        Key::Num5 => Some('5'),
        Key::Num6 => Some('6'),
        Key::Num7 => Some('7'),
        Key::Num8 => Some('8'),
        Key::Num9 => Some('9'),
        Key::Minus => Some('-'),
        Key::Equals => Some('='),
        Key::Period => Some('.'),
        Key::Comma => Some(','),
        Key::Semicolon => Some(';'),
        Key::Slash => Some('/'),
        Key::Backslash => Some('\\'),
        Key::Backtick => Some('`'),
        _ => None,
    }
}
