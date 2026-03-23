// Maps egui keyboard events to terminal input bytes.

use egui::{Event, Key, Modifiers};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputMode {
    pub app_cursor: bool,
    pub bracketed_paste: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct InputResult {
    pub bytes: Vec<u8>,
    pub copy_selection: bool,
}

/// Process egui input events and return terminal actions.
pub fn process_input(
    ctx: &egui::Context,
    void_shortcuts: &[(Modifiers, Key)],
    mode: InputMode,
    has_selection: bool,
) -> InputResult {
    let mut output = InputResult::default();

    ctx.input(|input| {
        // Check if a paste event exists (to avoid double-processing with Ctrl+V raw byte)
        let has_paste_event = input
            .events
            .iter()
            .any(|e| matches!(e, Event::Paste(_)));

        // Check if any Ctrl/Alt key event exists this frame — if so, skip Text events
        // to avoid sending raw characters alongside control sequences (e.g., Ctrl+W
        // sending both 'w' text and \x17 key).
        let has_ctrl_key = input.events.iter().any(|e| matches!(e, Event::Key {
            pressed: true,
            modifiers: Modifiers { ctrl: true, .. } | Modifiers { alt: true, .. },
            ..
        }));

        for event in &input.events {
            match event {
                Event::Text(text) => {
                    // Skip text events when Ctrl/Alt key events are present (avoids
                    // double-send on Windows where WM_CHAR fires alongside WM_KEYDOWN).
                    if has_ctrl_key {
                        continue;
                    }
                    if !input.modifiers.ctrl && !input.modifiers.alt {
                        output.bytes.extend_from_slice(text.as_bytes());
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if is_void_shortcut(modifiers, key, void_shortcuts) {
                        continue;
                    }

                    if should_copy_selection(modifiers, key, has_selection) {
                        output.copy_selection = true;
                        continue;
                    }

                    // Skip raw Ctrl+V (0x16) when clipboard paste event is present
                    if modifiers.ctrl
                        && !modifiers.shift
                        && !modifiers.alt
                        && *key == Key::V
                        && has_paste_event
                    {
                        continue;
                    }

                    if let Some(bytes) = key_to_bytes(key, modifiers, mode) {
                        output.bytes.extend_from_slice(&bytes);
                    }
                }
                Event::Paste(text) => {
                    if mode.bracketed_paste {
                        output.bytes.extend_from_slice(b"\x1b[200~");
                        output.bytes.extend_from_slice(text.as_bytes());
                        output.bytes.extend_from_slice(b"\x1b[201~");
                    } else {
                        output.bytes.extend_from_slice(text.as_bytes());
                    }
                }
                Event::Copy => handle_copy_event(&mut output, has_selection),
                Event::Cut => handle_cut_event(&mut output, has_selection),
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

#[cfg(target_os = "macos")]
fn should_copy_selection(modifiers: &Modifiers, key: &Key, has_selection: bool) -> bool {
    has_selection && modifiers.command && !modifiers.ctrl && !modifiers.alt && *key == Key::C
}

#[cfg(not(target_os = "macos"))]
fn should_copy_selection(modifiers: &Modifiers, key: &Key, has_selection: bool) -> bool {
    (modifiers.ctrl && modifiers.shift && !modifiers.alt && *key == Key::C)
        || (has_selection && modifiers.ctrl && !modifiers.shift && !modifiers.alt && *key == Key::C)
}

#[cfg(target_os = "macos")]
fn handle_copy_event(output: &mut InputResult, has_selection: bool) {
    if has_selection {
        output.copy_selection = true;
    }
}

#[cfg(not(target_os = "macos"))]
fn handle_copy_event(output: &mut InputResult, has_selection: bool) {
    if has_selection {
        output.copy_selection = true;
    } else {
        output.bytes.push(0x03);
    }
}

#[cfg(target_os = "macos")]
fn handle_cut_event(_output: &mut InputResult, _has_selection: bool) {}

#[cfg(not(target_os = "macos"))]
fn handle_cut_event(output: &mut InputResult, has_selection: bool) {
    if !has_selection {
        output.bytes.push(0x18);
    }
}

/// Convert a key press + modifiers to terminal byte sequence.
fn key_to_bytes(key: &Key, modifiers: &Modifiers, mode: InputMode) -> Option<Vec<u8>> {
    // Ctrl+letter -> control character.
    if modifiers.ctrl && !modifiers.shift && !modifiers.alt {
        if let Some(byte) = ctrl_key_byte(key) {
            return Some(vec![byte]);
        }
    }

    // Alt+key -> ESC prefix.
    if modifiers.alt && !modifiers.ctrl {
        if let Some(c) = key_to_char(key) {
            let mut bytes = vec![0x1b];
            bytes.extend_from_slice(c.to_string().as_bytes());
            return Some(bytes);
        }
    }

    match key {
        Key::Enter => {
            if modifiers.alt {
                Some(b"\x1b\r".to_vec()) // Alt+Enter: ESC + CR
            } else if modifiers.shift {
                Some(b"\n".to_vec()) // Shift+Enter: newline (multiline in modern shells)
            } else {
                Some(b"\r".to_vec()) // Enter: carriage return
            }
        }
        Key::Backspace => {
            if modifiers.ctrl {
                Some(b"\x17".to_vec()) // Ctrl+W — delete word (readline standard)
            } else if modifiers.alt {
                Some(b"\x1b\x7f".to_vec()) // Alt+Backspace — delete word
            } else {
                Some(b"\x7f".to_vec())
            }
        }
        Key::Tab => {
            if modifiers.shift {
                Some(b"\x1b[Z".to_vec())
            } else {
                Some(b"\t".to_vec())
            }
        }
        Key::Escape => Some(b"\x1b".to_vec()),
        Key::ArrowUp => Some(cursor_key_sequence(
            b"A",
            b"\x1bOA",
            modifiers,
            mode.app_cursor,
        )),
        Key::ArrowDown => Some(cursor_key_sequence(
            b"B",
            b"\x1bOB",
            modifiers,
            mode.app_cursor,
        )),
        Key::ArrowRight => Some(cursor_key_sequence(
            b"C",
            b"\x1bOC",
            modifiers,
            mode.app_cursor,
        )),
        Key::ArrowLeft => Some(cursor_key_sequence(
            b"D",
            b"\x1bOD",
            modifiers,
            mode.app_cursor,
        )),
        Key::Home => Some(cursor_key_sequence(
            b"H",
            b"\x1bOH",
            modifiers,
            mode.app_cursor,
        )),
        Key::End => Some(cursor_key_sequence(
            b"F",
            b"\x1bOF",
            modifiers,
            mode.app_cursor,
        )),
        Key::PageUp => Some(tilde_key_with_mods(b"5", modifiers)),
        Key::PageDown => Some(tilde_key_with_mods(b"6", modifiers)),
        Key::Insert => Some(tilde_key_with_mods(b"2", modifiers)),
        Key::Delete => {
            if modifiers.ctrl {
                Some(b"\x1bd".to_vec()) // Ctrl+Delete: ESC d — readline delete-word-forward
            } else {
                Some(tilde_key_with_mods(b"3", modifiers))
            }
        }
        Key::F1 => Some(fkey_sequence(b"P", b"11", modifiers, true)),
        Key::F2 => Some(fkey_sequence(b"Q", b"12", modifiers, true)),
        Key::F3 => Some(fkey_sequence(b"R", b"13", modifiers, true)),
        Key::F4 => Some(fkey_sequence(b"S", b"14", modifiers, true)),
        Key::F5 => Some(fkey_sequence(b"15", b"15", modifiers, false)),
        Key::F6 => Some(fkey_sequence(b"17", b"17", modifiers, false)),
        Key::F7 => Some(fkey_sequence(b"18", b"18", modifiers, false)),
        Key::F8 => Some(fkey_sequence(b"19", b"19", modifiers, false)),
        Key::F9 => Some(fkey_sequence(b"20", b"20", modifiers, false)),
        Key::F10 => Some(fkey_sequence(b"21", b"21", modifiers, false)),
        Key::F11 => Some(fkey_sequence(b"23", b"23", modifiers, false)),
        Key::F12 => Some(fkey_sequence(b"24", b"24", modifiers, false)),
        Key::F13 => Some(fkey_sequence(b"25", b"25", modifiers, false)),
        Key::F14 => Some(fkey_sequence(b"26", b"26", modifiers, false)),
        Key::F15 => Some(fkey_sequence(b"28", b"28", modifiers, false)),
        Key::F16 => Some(fkey_sequence(b"29", b"29", modifiers, false)),
        Key::F17 => Some(fkey_sequence(b"31", b"31", modifiers, false)),
        Key::F18 => Some(fkey_sequence(b"32", b"32", modifiers, false)),
        Key::F19 => Some(fkey_sequence(b"33", b"33", modifiers, false)),
        Key::F20 => Some(fkey_sequence(b"34", b"34", modifiers, false)),
        Key::Space => {
            if modifiers.ctrl {
                Some(vec![0x00])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// CSI tilde key with optional modifier: \x1b[num~ or \x1b[num;mod~
fn tilde_key_with_mods(num: &[u8], modifiers: &Modifiers) -> Vec<u8> {
    let m = modifier_param(modifiers);
    let mut seq = b"\x1b[".to_vec();
    seq.extend_from_slice(num);
    if m > 1 {
        seq.push(b';');
        seq.extend_from_slice(m.to_string().as_bytes());
    }
    seq.push(b'~');
    seq
}

/// Generate F-key sequence with optional modifier encoding.
/// F1-F4 use SS3 format (\x1bOP) without modifiers, CSI format (\x1b[1;modP) with modifiers.
/// F5+ use CSI tilde format (\x1b[15~) without modifiers, (\x1b[15;mod~) with modifiers.
fn fkey_sequence(
    suffix: &[u8],
    csi_num: &[u8],
    modifiers: &Modifiers,
    is_ss3: bool,
) -> Vec<u8> {
    let m = modifier_param(modifiers);
    if m == 1 {
        // No modifiers
        if is_ss3 {
            // SS3 format: \x1bOP
            let mut seq = b"\x1bO".to_vec();
            seq.extend_from_slice(suffix);
            seq
        } else {
            // CSI tilde format: \x1b[15~
            let mut seq = b"\x1b[".to_vec();
            seq.extend_from_slice(csi_num);
            seq.push(b'~');
            seq
        }
    } else {
        // With modifiers: always CSI format \x1b[num;modX or \x1b[1;modX
        let mut seq = b"\x1b[".to_vec();
        if is_ss3 {
            seq.extend_from_slice(b"1;");
        } else {
            seq.extend_from_slice(csi_num);
            seq.push(b';');
        }
        seq.extend_from_slice(m.to_string().as_bytes());
        if is_ss3 {
            seq.extend_from_slice(suffix);
        } else {
            seq.push(b'~');
        }
        seq
    }
}

fn cursor_key_sequence(
    normal_suffix: &[u8],
    app_sequence: &[u8],
    modifiers: &Modifiers,
    app_cursor: bool,
) -> Vec<u8> {
    if app_cursor && modifier_param(modifiers) == 1 {
        app_sequence.to_vec()
    } else {
        csi_modifier(normal_suffix, modifiers)
    }
}

/// Generate CSI sequence with modifier encoding for arrow/nav keys.
fn csi_modifier(suffix: &[u8], modifiers: &Modifiers) -> Vec<u8> {
    let modifier_code = modifier_param(modifiers);
    if modifier_code > 1 {
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

/// Map Ctrl+key to control character byte (0x01-0x1A for A-Z).
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
        Key::OpenBracket => Some(0x1B),
        Key::Backslash => Some(0x1C),
        Key::CloseBracket => Some(0x1D),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_keys_follow_application_cursor_mode() {
        let modifiers = Modifiers::default();

        assert_eq!(
            key_to_bytes(
                &Key::ArrowUp,
                &modifiers,
                InputMode {
                    app_cursor: true,
                    bracketed_paste: false,
                },
            ),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            key_to_bytes(
                &Key::ArrowUp,
                &modifiers,
                InputMode {
                    app_cursor: false,
                    bracketed_paste: false,
                },
            ),
            Some(b"\x1b[A".to_vec())
        );
    }

    #[test]
    fn modified_arrow_keys_stay_in_csi_form() {
        let modifiers = Modifiers {
            shift: true,
            ..Modifiers::default()
        };

        assert_eq!(
            key_to_bytes(
                &Key::ArrowRight,
                &modifiers,
                InputMode {
                    app_cursor: true,
                    bracketed_paste: false,
                },
            ),
            Some(b"\x1b[1;2C".to_vec())
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn copy_event_maps_to_sigint_on_non_macos() {
        let mut result = InputResult::default();
        handle_copy_event(&mut result, false);

        assert_eq!(result.bytes, vec![0x03]);
        assert!(!result.copy_selection);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn copy_event_prefers_selection_over_sigint() {
        let mut result = InputResult::default();
        handle_copy_event(&mut result, true);

        assert!(result.bytes.is_empty());
        assert!(result.copy_selection);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn ctrl_c_with_selection_is_copy_shortcut() {
        let modifiers = Modifiers {
            ctrl: true,
            ..Modifiers::default()
        };

        assert!(should_copy_selection(&modifiers, &Key::C, true));
        assert!(!should_copy_selection(&modifiers, &Key::C, false));
    }
}
