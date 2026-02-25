//! TUI color themes.
//!
//! Each theme defines a set of named color slots used by `ui.rs` when rendering
//! the terminal interface.  New themes can be added by extending `get_theme()`.

use ratatui::style::Color;

/// Named color slots used throughout the TUI renderer.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    /// Primary accent — AI labels, selected items.
    pub primary: Color,
    /// Secondary accent — system messages, file headers.
    pub secondary: Color,
    /// Success / additions.
    pub success: Color,
    /// Errors / deletions.
    pub error: Color,
    /// Warnings / pending-approval banners.
    pub warning: Color,
    /// Info lines — diff context, diff headers.
    pub info: Color,
    /// Dimmed text — step output, metadata.
    pub dim: Color,
    /// Normal body text.
    pub text: Color,
    /// Accent color — file list icons, diffs.
    pub accent: Color,
    /// Logo / banner color.
    pub logo: Color,
    /// File-tree selection foreground.
    pub selection_fg: Color,
    /// File-tree selection background.
    pub selection_bg: Color,
}

// ── Built-in themes ───────────────────────────────────────────────────────────

const DARK: Theme = Theme {
    name: "dark",
    primary: Color::Cyan,
    secondary: Color::Yellow,
    success: Color::Green,
    error: Color::Red,
    warning: Color::Yellow,
    info: Color::Cyan,
    dim: Color::DarkGray,
    text: Color::White,
    accent: Color::Blue,
    logo: Color::Rgb(255, 100, 100),
    selection_fg: Color::Black,
    selection_bg: Color::Blue,
};

const LIGHT: Theme = Theme {
    name: "light",
    primary: Color::Blue,
    secondary: Color::Magenta,
    success: Color::Green,
    error: Color::Red,
    warning: Color::Magenta,
    info: Color::Blue,
    dim: Color::Gray,
    text: Color::Black,
    accent: Color::Magenta,
    logo: Color::Rgb(180, 60, 60),
    selection_fg: Color::White,
    selection_bg: Color::Blue,
};

const MONOKAI: Theme = Theme {
    name: "monokai",
    primary: Color::Rgb(102, 217, 239),   // cyan
    secondary: Color::Rgb(249, 38, 114),  // pink
    success: Color::Rgb(166, 226, 46),    // green
    error: Color::Rgb(249, 38, 114),      // pink
    warning: Color::Rgb(230, 219, 116),   // yellow
    info: Color::Rgb(102, 217, 239),      // cyan
    dim: Color::Rgb(117, 113, 94),        // comment gray
    text: Color::Rgb(248, 248, 242),      // off-white
    accent: Color::Rgb(174, 129, 255),    // purple
    logo: Color::Rgb(249, 38, 114),       // pink
    selection_fg: Color::Rgb(30, 30, 30),
    selection_bg: Color::Rgb(174, 129, 255),
};

const SOLARIZED: Theme = Theme {
    name: "solarized",
    primary: Color::Rgb(38, 139, 210),    // blue
    secondary: Color::Rgb(181, 137, 0),   // yellow
    success: Color::Rgb(133, 153, 0),     // green
    error: Color::Rgb(220, 50, 47),       // red
    warning: Color::Rgb(203, 75, 22),     // orange
    info: Color::Rgb(42, 161, 152),       // cyan
    dim: Color::Rgb(88, 110, 117),        // base01
    text: Color::Rgb(131, 148, 150),      // base0
    accent: Color::Rgb(108, 113, 196),    // violet
    logo: Color::Rgb(38, 139, 210),       // blue
    selection_fg: Color::Rgb(0, 43, 54),  // base03
    selection_bg: Color::Rgb(38, 139, 210),
};

const NORD: Theme = Theme {
    name: "nord",
    primary: Color::Rgb(136, 192, 208),   // nord8
    secondary: Color::Rgb(235, 203, 139), // nord13 yellow
    success: Color::Rgb(163, 190, 140),   // nord14 green
    error: Color::Rgb(191, 97, 106),      // nord11 red
    warning: Color::Rgb(208, 135, 112),   // nord12 orange
    info: Color::Rgb(129, 161, 193),      // nord9
    dim: Color::Rgb(76, 86, 106),         // nord3
    text: Color::Rgb(216, 222, 233),      // nord4
    accent: Color::Rgb(180, 142, 173),    // nord15 purple
    logo: Color::Rgb(136, 192, 208),      // nord8
    selection_fg: Color::Rgb(36, 41, 51), // nord0
    selection_bg: Color::Rgb(136, 192, 208),
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Return the names of all available themes.
pub fn available_themes() -> &'static [&'static str] {
    &["dark", "light", "monokai", "solarized", "nord"]
}

/// Return the `Theme` for the given name, or the default dark theme if unknown.
pub fn get_theme(name: &str) -> Theme {
    match name {
        "light"     => LIGHT,
        "monokai"   => MONOKAI,
        "solarized" => SOLARIZED,
        "nord"      => NORD,
        _           => DARK,
    }
}
