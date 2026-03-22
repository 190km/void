// Color mapping: alacritty_terminal Color → egui Color32

use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, NamedColor};
use egui::Color32;

/// Default ANSI 16-color palette.
const ANSI_COLORS: [Color32; 16] = [
    Color32::from_rgb(0, 0, 0),       // 0  Black
    Color32::from_rgb(204, 0, 0),     // 1  Red
    Color32::from_rgb(78, 154, 6),    // 2  Green
    Color32::from_rgb(196, 160, 0),   // 3  Yellow
    Color32::from_rgb(52, 101, 164),  // 4  Blue
    Color32::from_rgb(117, 80, 123),  // 5  Magenta
    Color32::from_rgb(6, 152, 154),   // 6  Cyan
    Color32::from_rgb(211, 215, 207), // 7  White
    Color32::from_rgb(85, 87, 83),    // 8  Bright Black
    Color32::from_rgb(239, 41, 41),   // 9  Bright Red
    Color32::from_rgb(138, 226, 52),  // 10 Bright Green
    Color32::from_rgb(252, 233, 79),  // 11 Bright Yellow
    Color32::from_rgb(114, 159, 207), // 12 Bright Blue
    Color32::from_rgb(173, 127, 168), // 13 Bright Magenta
    Color32::from_rgb(52, 226, 226),  // 14 Bright Cyan
    Color32::from_rgb(238, 238, 236), // 15 Bright White
];

pub const DEFAULT_FG: Color32 = Color32::from_rgb(200, 200, 200);
pub const DEFAULT_BG: Color32 = Color32::from_rgb(17, 17, 17);

/// Convert an alacritty Color to an egui Color32.
pub fn to_egui_color(color: Color, colors: &Colors) -> Color32 {
    match color {
        Color::Named(named) => named_to_egui(named, colors),
        Color::Spec(rgb) => Color32::from_rgb(rgb.r, rgb.g, rgb.b),
        Color::Indexed(idx) => indexed_to_egui(idx, colors),
    }
}

fn named_to_egui(named: NamedColor, colors: &Colors) -> Color32 {
    // Check for custom color override first
    if let Some(rgb) = colors[named] {
        return Color32::from_rgb(rgb.r, rgb.g, rgb.b);
    }

    match named {
        NamedColor::Black => ANSI_COLORS[0],
        NamedColor::Red => ANSI_COLORS[1],
        NamedColor::Green => ANSI_COLORS[2],
        NamedColor::Yellow => ANSI_COLORS[3],
        NamedColor::Blue => ANSI_COLORS[4],
        NamedColor::Magenta => ANSI_COLORS[5],
        NamedColor::Cyan => ANSI_COLORS[6],
        NamedColor::White => ANSI_COLORS[7],
        NamedColor::BrightBlack => ANSI_COLORS[8],
        NamedColor::BrightRed => ANSI_COLORS[9],
        NamedColor::BrightGreen => ANSI_COLORS[10],
        NamedColor::BrightYellow => ANSI_COLORS[11],
        NamedColor::BrightBlue => ANSI_COLORS[12],
        NamedColor::BrightMagenta => ANSI_COLORS[13],
        NamedColor::BrightCyan => ANSI_COLORS[14],
        NamedColor::BrightWhite => ANSI_COLORS[15],
        NamedColor::Foreground | NamedColor::BrightForeground => DEFAULT_FG,
        NamedColor::Background => DEFAULT_BG,
        NamedColor::Cursor => DEFAULT_FG,
        NamedColor::DimBlack => dim_color(ANSI_COLORS[0]),
        NamedColor::DimRed => dim_color(ANSI_COLORS[1]),
        NamedColor::DimGreen => dim_color(ANSI_COLORS[2]),
        NamedColor::DimYellow => dim_color(ANSI_COLORS[3]),
        NamedColor::DimBlue => dim_color(ANSI_COLORS[4]),
        NamedColor::DimMagenta => dim_color(ANSI_COLORS[5]),
        NamedColor::DimCyan => dim_color(ANSI_COLORS[6]),
        NamedColor::DimWhite => dim_color(ANSI_COLORS[7]),
        NamedColor::DimForeground => dim_color(DEFAULT_FG),
    }
}

fn indexed_to_egui(idx: u8, colors: &Colors) -> Color32 {
    // Custom override
    if let Some(rgb) = colors[idx as usize] {
        return Color32::from_rgb(rgb.r, rgb.g, rgb.b);
    }

    if idx < 16 {
        ANSI_COLORS[idx as usize]
    } else if idx < 232 {
        // 6×6×6 color cube (indices 16–231)
        let i = idx - 16;
        let r = (i / 36) % 6;
        let g = (i / 6) % 6;
        let b = i % 6;
        let r = if r > 0 { r * 40 + 55 } else { 0 };
        let g = if g > 0 { g * 40 + 55 } else { 0 };
        let b = if b > 0 { b * 40 + 55 } else { 0 };
        Color32::from_rgb(r, g, b)
    } else {
        // Grayscale ramp (indices 232–255)
        let v = (idx - 232) * 10 + 8;
        Color32::from_rgb(v, v, v)
    }
}

fn dim_color(c: Color32) -> Color32 {
    Color32::from_rgb(
        (c.r() as u16 * 2 / 3) as u8,
        (c.g() as u16 * 2 / 3) as u8,
        (c.b() as u16 * 2 / 3) as u8,
    )
}
