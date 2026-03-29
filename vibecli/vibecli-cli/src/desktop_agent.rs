//! Desktop GUI Automation — cross-platform mouse, keyboard, and window
//! management for VibeCody's agent framework.
//!
//! Shells out to platform-specific CLI tools:
//! - **macOS**: `osascript`, `cliclick`, `screencapture`
//! - **Linux**: `xdotool`, `wmctrl`, `scrot`, `xdpyinfo`
//! - **Windows**: PowerShell (`System.Windows.Forms`, `user32.dll`)
//!
//! No additional Rust crate dependencies required.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{debug, info, warn};

// ── Platform Detection ──────────────────────────────────────────────────────

/// Supported desktop platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopPlatform {
    MacOS,
    Linux,
    Windows,
}

impl fmt::Display for DesktopPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MacOS => write!(f, "macOS"),
            Self::Linux => write!(f, "Linux"),
            Self::Windows => write!(f, "Windows"),
        }
    }
}

/// Detect the current desktop platform at runtime.
pub fn detect_platform() -> DesktopPlatform {
    if cfg!(target_os = "macos") {
        DesktopPlatform::MacOS
    } else if cfg!(target_os = "windows") {
        DesktopPlatform::Windows
    } else {
        DesktopPlatform::Linux
    }
}

// ── Mouse Button ────────────────────────────────────────────────────────────

/// Mouse button for click actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl fmt::Display for MouseButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => write!(f, "left"),
            Self::Right => write!(f, "right"),
            Self::Middle => write!(f, "middle"),
        }
    }
}

impl MouseButton {
    /// X11 button number for xdotool.
    fn xdotool_button(&self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Right => 3,
            Self::Middle => 2,
        }
    }

    /// cliclick button character.
    fn cliclick_char(&self) -> char {
        match self {
            Self::Left => 'c',
            Self::Right => 'r', // rc: = right-click in cliclick syntax (handled in caller)
            Self::Middle => 'c', // cliclick doesn't natively support middle; fall back to left
        }
    }
}

// ── Desktop Actions ─────────────────────────────────────────────────────────

/// A discrete desktop automation action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DesktopAction {
    /// Move mouse cursor to absolute coordinates.
    MoveMouse { x: u32, y: u32 },
    /// Click a mouse button at coordinates.
    Click {
        button: MouseButton,
        x: u32,
        y: u32,
    },
    /// Double-click (left button) at coordinates.
    DoubleClick { x: u32, y: u32 },
    /// Drag from one point to another.
    Drag {
        from_x: u32,
        from_y: u32,
        to_x: u32,
        to_y: u32,
    },
    /// Type a text string.
    TypeText { text: String },
    /// Press a single key (e.g., "Return", "Escape", "Tab").
    PressKey { key: String },
    /// Press a key combination (e.g., modifiers=["ctrl"], key="c").
    KeyCombo {
        modifiers: Vec<String>,
        key: String,
    },
    /// Capture a screenshot to the given path.
    Screenshot { path: String },
    /// Get the currently active/focused window.
    GetActiveWindow,
    /// Focus a window whose title matches the pattern.
    FocusWindow { title_pattern: String },
    /// List all visible windows.
    ListWindows,
    /// Set the active window's size.
    SetWindowSize { width: u32, height: u32 },
    /// Get the primary screen dimensions.
    GetScreenSize,
    /// Get current mouse cursor position.
    GetMousePosition,
    /// Pause for the specified duration.
    Delay { ms: u64 },
}

impl fmt::Display for DesktopAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MoveMouse { x, y } => write!(f, "MoveMouse({x}, {y})"),
            Self::Click { button, x, y } => write!(f, "Click({button}, {x}, {y})"),
            Self::DoubleClick { x, y } => write!(f, "DoubleClick({x}, {y})"),
            Self::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
            } => write!(f, "Drag({from_x},{from_y} -> {to_x},{to_y})"),
            Self::TypeText { text } => write!(f, "TypeText({text:?})"),
            Self::PressKey { key } => write!(f, "PressKey({key})"),
            Self::KeyCombo { modifiers, key } => {
                write!(f, "KeyCombo({}+{key})", modifiers.join("+"))
            }
            Self::Screenshot { path } => write!(f, "Screenshot({path})"),
            Self::GetActiveWindow => write!(f, "GetActiveWindow"),
            Self::FocusWindow { title_pattern } => write!(f, "FocusWindow({title_pattern:?})"),
            Self::ListWindows => write!(f, "ListWindows"),
            Self::SetWindowSize { width, height } => {
                write!(f, "SetWindowSize({width}x{height})")
            }
            Self::GetScreenSize => write!(f, "GetScreenSize"),
            Self::GetMousePosition => write!(f, "GetMousePosition"),
            Self::Delay { ms } => write!(f, "Delay({ms}ms)"),
        }
    }
}

// ── Result Types ────────────────────────────────────────────────────────────

/// Information about a desktop window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub focused: bool,
}

/// Screen/display dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl Default for ScreenInfo {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
        }
    }
}

/// Result of executing a desktop action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopResult {
    pub success: bool,
    pub output: String,
    pub windows: Option<Vec<WindowInfo>>,
    pub screen: Option<ScreenInfo>,
    pub mouse_pos: Option<(u32, u32)>,
}

impl DesktopResult {
    /// Create a successful result with a message.
    fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            windows: None,
            screen: None,
            mouse_pos: None,
        }
    }

    /// Create a failed result with an error message.
    fn fail(output: impl Into<String>) -> Self {
        Self {
            success: false,
            output: output.into(),
            windows: None,
            screen: None,
            mouse_pos: None,
        }
    }
}

// ── Shell Helpers ───────────────────────────────────────────────────────────

/// Escape a string for safe inclusion in a single-quoted shell argument.
/// Replaces `'` with `'\''` (end quote, escaped quote, start quote).
pub fn shell_escape(s: &str) -> String {
    s.replace('\'', "'\\''")
}

/// Map a friendly key name to an osascript key name.
pub fn key_name_to_osascript(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "return" | "enter" => "return".to_string(),
        "escape" | "esc" => "escape".to_string(),
        "tab" => "tab".to_string(),
        "space" => "space".to_string(),
        "delete" | "backspace" => "delete".to_string(),
        "forwarddelete" | "del" => "forward delete".to_string(),
        "up" | "uparrow" => "up arrow".to_string(),
        "down" | "downarrow" => "down arrow".to_string(),
        "left" | "leftarrow" => "left arrow".to_string(),
        "right" | "rightarrow" => "right arrow".to_string(),
        "home" => "home".to_string(),
        "end" => "end".to_string(),
        "pageup" => "page up".to_string(),
        "pagedown" => "page down".to_string(),
        "f1" => "F1".to_string(),
        "f2" => "F2".to_string(),
        "f3" => "F3".to_string(),
        "f4" => "F4".to_string(),
        "f5" => "F5".to_string(),
        "f6" => "F6".to_string(),
        "f7" => "F7".to_string(),
        "f8" => "F8".to_string(),
        "f9" => "F9".to_string(),
        "f10" => "F10".to_string(),
        "f11" => "F11".to_string(),
        "f12" => "F12".to_string(),
        other => other.to_string(),
    }
}

/// Map a friendly key name to an xdotool key name.
pub fn key_name_to_xdotool(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "return" | "enter" => "Return".to_string(),
        "escape" | "esc" => "Escape".to_string(),
        "tab" => "Tab".to_string(),
        "space" => "space".to_string(),
        "delete" | "backspace" => "BackSpace".to_string(),
        "forwarddelete" | "del" => "Delete".to_string(),
        "up" | "uparrow" => "Up".to_string(),
        "down" | "downarrow" => "Down".to_string(),
        "left" | "leftarrow" => "Left".to_string(),
        "right" | "rightarrow" => "Right".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "pageup" => "Prior".to_string(),
        "pagedown" => "Next".to_string(),
        "f1" => "F1".to_string(),
        "f2" => "F2".to_string(),
        "f3" => "F3".to_string(),
        "f4" => "F4".to_string(),
        "f5" => "F5".to_string(),
        "f6" => "F6".to_string(),
        "f7" => "F7".to_string(),
        "f8" => "F8".to_string(),
        "f9" => "F9".to_string(),
        "f10" => "F10".to_string(),
        "f11" => "F11".to_string(),
        "f12" => "F12".to_string(),
        other => other.to_string(),
    }
}

/// Map a modifier name to the osascript "using … down" keyword.
pub fn modifier_to_osascript(m: &str) -> &str {
    match m.to_lowercase().as_str() {
        "ctrl" | "control" => "control",
        "alt" | "option" => "option",
        "cmd" | "command" | "super" | "meta" => "command",
        "shift" => "shift",
        _ => "control", // default fallback
    }
}

/// Map a modifier name for xdotool key combos.
fn modifier_to_xdotool(m: &str) -> &str {
    match m.to_lowercase().as_str() {
        "ctrl" | "control" => "ctrl",
        "alt" | "option" => "alt",
        "cmd" | "command" | "super" | "meta" => "super",
        "shift" => "shift",
        _ => "ctrl",
    }
}

/// Map a modifier name for PowerShell SendKeys syntax.
fn modifier_to_powershell(m: &str) -> &str {
    match m.to_lowercase().as_str() {
        "ctrl" | "control" => "^",
        "alt" | "option" => "%",
        "shift" => "+",
        // Windows doesn't have a "super" in SendKeys; map to ctrl
        "cmd" | "command" | "super" | "meta" => "^",
        _ => "^",
    }
}

// ── Window / Screen Parsers ─────────────────────────────────────────────────

/// Parse a window list from platform-specific command output.
pub fn parse_window_list(output: &str, platform: DesktopPlatform) -> Vec<WindowInfo> {
    match platform {
        DesktopPlatform::MacOS => parse_window_list_macos(output),
        DesktopPlatform::Linux => parse_window_list_linux(output),
        DesktopPlatform::Windows => parse_window_list_windows(output),
    }
}

/// Parse macOS AppleScript window list output.
///
/// Expected format from osascript (one line per window):
/// `<app_name> | <window_title> | <window_id>`
fn parse_window_list_macos(output: &str) -> Vec<WindowInfo> {
    let mut windows = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line == "missing value" {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        if parts.len() >= 2 {
            windows.push(WindowInfo {
                id: parts.get(2).unwrap_or(&"").to_string(),
                title: parts[1].to_string(),
                app: parts[0].to_string(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                focused: false,
            });
        } else {
            // Single-part line: treat as title
            windows.push(WindowInfo {
                id: String::new(),
                title: line.to_string(),
                app: String::new(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                focused: false,
            });
        }
    }
    windows
}

/// Parse Linux `wmctrl -l` output.
///
/// Format: `0x01234567  0 hostname Window Title Here`
fn parse_window_list_linux(output: &str) -> Vec<WindowInfo> {
    let mut windows = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // wmctrl -l format: id desktop hostname title...
        let parts: Vec<&str> = line.splitn(4, char::is_whitespace).collect();
        if parts.len() >= 4 {
            // parts[0] = window id, parts[1] = desktop, parts[2] = hostname, parts[3..] = title
            // Re-split more carefully to handle multiple spaces
            let mut iter = line.split_whitespace();
            let id = iter.next().unwrap_or("").to_string();
            let _desktop = iter.next().unwrap_or("");
            let hostname = iter.next().unwrap_or("");
            let title: String = iter.collect::<Vec<&str>>().join(" ");

            windows.push(WindowInfo {
                id,
                title,
                app: hostname.to_string(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                focused: false,
            });
        }
    }
    windows
}

/// Parse Windows PowerShell `Get-Process` window list output.
///
/// Expected format: `ProcessName | MainWindowTitle | Id`
fn parse_window_list_windows(output: &str) -> Vec<WindowInfo> {
    let mut windows = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        if parts.len() >= 2 {
            windows.push(WindowInfo {
                id: parts.get(2).unwrap_or(&"").to_string(),
                title: parts[1].to_string(),
                app: parts[0].to_string(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                focused: false,
            });
        }
    }
    windows
}

/// Parse screen size from platform-specific output.
pub fn parse_screen_size(output: &str, platform: DesktopPlatform) -> Option<ScreenInfo> {
    match platform {
        DesktopPlatform::MacOS => parse_screen_size_macos(output),
        DesktopPlatform::Linux => parse_screen_size_linux(output),
        DesktopPlatform::Windows => parse_screen_size_windows(output),
    }
}

/// Parse macOS `system_profiler SPDisplaysDataType` or osascript output.
///
/// Looks for `Resolution: WxH` pattern.
fn parse_screen_size_macos(output: &str) -> Option<ScreenInfo> {
    // Try "Resolution: 2560 x 1440" pattern from system_profiler
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Resolution:") || line.contains("Resolution:") {
            let after = line.split("Resolution:").nth(1)?.trim();
            let parts: Vec<&str> = after.split('x').collect();
            if parts.len() >= 2 {
                let w = parts[0].trim().parse::<u32>().ok()?;
                let h = parts[1]
                    .split_whitespace()
                    .next()?
                    .parse::<u32>()
                    .ok()?;
                let scale = if line.contains("Retina") { 2.0 } else { 1.0 };
                return Some(ScreenInfo {
                    width: w,
                    height: h,
                    scale_factor: scale,
                });
            }
        }
    }
    // Try comma-separated bounds: "0, 0, 1440, 900"
    let nums: Vec<u32> = output
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect();
    if nums.len() >= 4 {
        return Some(ScreenInfo {
            width: nums[2],
            height: nums[3],
            scale_factor: 1.0,
        });
    }
    None
}

/// Parse Linux `xdpyinfo | grep dimensions` output.
///
/// Format: `  dimensions:    1920x1080 pixels (508x285 millimeters)`
fn parse_screen_size_linux(output: &str) -> Option<ScreenInfo> {
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("dimensions:") || line.contains("dimensions:") {
            // Extract "1920x1080"
            let after = line.split("dimensions:").nth(1)?.trim();
            let dim_str = after.split_whitespace().next()?;
            let parts: Vec<&str> = dim_str.split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].parse::<u32>().ok()?;
                let h = parts[1].parse::<u32>().ok()?;
                return Some(ScreenInfo {
                    width: w,
                    height: h,
                    scale_factor: 1.0,
                });
            }
        }
    }
    None
}

/// Parse Windows PowerShell screen size output.
///
/// Expected: `Width | Height`
fn parse_screen_size_windows(output: &str) -> Option<ScreenInfo> {
    let line = output.lines().next()?.trim();
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() >= 2 {
        let w = parts[0].trim().parse::<u32>().ok()?;
        let h = parts[1].trim().parse::<u32>().ok()?;
        return Some(ScreenInfo {
            width: w,
            height: h,
            scale_factor: 1.0,
        });
    }
    None
}

/// Parse mouse position from platform-specific output.
pub fn parse_mouse_position(output: &str, platform: DesktopPlatform) -> Option<(u32, u32)> {
    match platform {
        DesktopPlatform::MacOS => parse_mouse_position_macos(output),
        DesktopPlatform::Linux => parse_mouse_position_linux(output),
        DesktopPlatform::Windows => parse_mouse_position_windows(output),
    }
}

/// Parse macOS mouse position output (osascript returns `{x, y}`).
fn parse_mouse_position_macos(output: &str) -> Option<(u32, u32)> {
    let trimmed = output.trim().trim_matches('{').trim_matches('}');
    let parts: Vec<&str> = trimmed.split(',').collect();
    if parts.len() >= 2 {
        let x = parts[0].trim().parse::<u32>().ok()?;
        let y = parts[1].trim().parse::<u32>().ok()?;
        return Some((x, y));
    }
    None
}

/// Parse Linux `xdotool getmouselocation` output.
///
/// Format: `x:1234 y:567 screen:0 window:12345678`
fn parse_mouse_position_linux(output: &str) -> Option<(u32, u32)> {
    let mut x: Option<u32> = None;
    let mut y: Option<u32> = None;
    for part in output.split_whitespace() {
        if let Some(val) = part.strip_prefix("x:") {
            x = val.parse().ok();
        } else if let Some(val) = part.strip_prefix("y:") {
            y = val.parse().ok();
        }
    }
    Some((x?, y?))
}

/// Parse Windows PowerShell cursor position output (`X | Y`).
fn parse_mouse_position_windows(output: &str) -> Option<(u32, u32)> {
    let line = output.lines().next()?.trim();
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() >= 2 {
        let x = parts[0].trim().parse::<u32>().ok()?;
        let y = parts[1].trim().parse::<u32>().ok()?;
        return Some((x, y));
    }
    None
}

// ── DesktopAutomation ───────────────────────────────────────────────────────

/// Cross-platform desktop automation engine.
///
/// Builds and executes shell commands for mouse, keyboard, and window
/// management. Each method delegates to the platform-appropriate CLI tool.
pub struct DesktopAutomation {
    pub platform: DesktopPlatform,
    /// Milliseconds to pause between sequential actions (default: 100).
    pub action_delay_ms: u64,
}

impl DesktopAutomation {
    /// Create a new automation engine for the current platform.
    pub fn new() -> Self {
        let platform = detect_platform();
        info!("DesktopAutomation initialized for {platform}");
        Self {
            platform,
            action_delay_ms: 100,
        }
    }

    /// Create an automation engine targeting a specific platform.
    pub fn for_platform(platform: DesktopPlatform) -> Self {
        Self {
            platform,
            action_delay_ms: 100,
        }
    }

    // ── Main dispatch ───────────────────────────────────────────────────

    /// Execute a single desktop action.
    pub async fn execute(&self, action: &DesktopAction) -> Result<DesktopResult> {
        debug!("Executing desktop action: {action}");
        match action {
            DesktopAction::MoveMouse { x, y } => self.move_mouse(*x, *y).await,
            DesktopAction::Click { button, x, y } => self.click(*button, *x, *y).await,
            DesktopAction::DoubleClick { x, y } => self.double_click(*x, *y).await,
            DesktopAction::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
            } => self.drag(*from_x, *from_y, *to_x, *to_y).await,
            DesktopAction::TypeText { text } => self.type_text(text).await,
            DesktopAction::PressKey { key } => self.press_key(key).await,
            DesktopAction::KeyCombo { modifiers, key } => self.key_combo(modifiers, key).await,
            DesktopAction::Screenshot { path } => self.screenshot(path).await,
            DesktopAction::GetActiveWindow => self.get_active_window().await,
            DesktopAction::FocusWindow { title_pattern } => {
                self.focus_window(title_pattern).await
            }
            DesktopAction::ListWindows => self.list_windows().await,
            DesktopAction::SetWindowSize { width, height } => {
                self.set_window_size(*width, *height).await
            }
            DesktopAction::GetScreenSize => self.get_screen_size().await,
            DesktopAction::GetMousePosition => self.get_mouse_position().await,
            DesktopAction::Delay { ms } => {
                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                Ok(DesktopResult::ok(format!("Delayed {ms}ms")))
            }
        }
    }

    /// Execute a sequence of actions with `action_delay_ms` between each.
    pub async fn execute_sequence(
        &self,
        actions: &[DesktopAction],
    ) -> Result<Vec<DesktopResult>> {
        let mut results = Vec::with_capacity(actions.len());
        for (i, action) in actions.iter().enumerate() {
            let result = self.execute(action).await?;
            results.push(result);
            // Insert delay between actions (but not after the last one)
            if i < actions.len() - 1 && self.action_delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.action_delay_ms)).await;
            }
        }
        Ok(results)
    }

    // ── Mouse ───────────────────────────────────────────────────────────

    async fn move_mouse(&self, x: u32, y: u32) -> Result<DesktopResult> {
        let cmd = self.build_move_mouse_cmd(x, y);
        run_shell(&cmd).await
    }

    /// Build the shell command string for moving the mouse (exposed for testing).
    pub fn build_move_mouse_cmd(&self, x: u32, y: u32) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                format!("cliclick m:{x},{y}")
            }
            DesktopPlatform::Linux => {
                format!("xdotool mousemove {x} {y}")
            }
            DesktopPlatform::Windows => {
                format!(
                    "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; \
                     [System.Windows.Forms.Cursor]::Position = \
                     New-Object System.Drawing.Point({x},{y})\""
                )
            }
        }
    }

    async fn click(&self, button: MouseButton, x: u32, y: u32) -> Result<DesktopResult> {
        let cmd = self.build_click_cmd(button, x, y);
        run_shell(&cmd).await
    }

    pub fn build_click_cmd(&self, button: MouseButton, x: u32, y: u32) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                let prefix = match button {
                    MouseButton::Left => "c",
                    MouseButton::Right => "rc",
                    MouseButton::Middle => "c", // cliclick has no middle-click; fall back
                };
                format!("cliclick {prefix}:{x},{y}")
            }
            DesktopPlatform::Linux => {
                let btn = button.xdotool_button();
                format!("xdotool mousemove {x} {y} click {btn}")
            }
            DesktopPlatform::Windows => {
                // Use PowerShell with System.Windows.Forms.Cursor + mouse_event
                let (down_flag, up_flag) = match button {
                    MouseButton::Left => ("0x0002", "0x0004"),
                    MouseButton::Right => ("0x0008", "0x0010"),
                    MouseButton::Middle => ("0x0020", "0x0040"),
                };
                format!(
                    "powershell -command \"\
                    Add-Type -AssemblyName System.Windows.Forms; \
                    [System.Windows.Forms.Cursor]::Position = \
                    New-Object System.Drawing.Point({x},{y}); \
                    $sig = '[DllImport(\\\"user32.dll\\\")] public static extern void mouse_event(int f,int x,int y,int d,int e);'; \
                    $m = Add-Type -MemberDefinition $sig -Name M -Namespace W -PassThru; \
                    $m::mouse_event({down_flag},0,0,0,0); \
                    $m::mouse_event({up_flag},0,0,0,0)\""
                )
            }
        }
    }

    async fn double_click(&self, x: u32, y: u32) -> Result<DesktopResult> {
        let cmd = self.build_double_click_cmd(x, y);
        run_shell(&cmd).await
    }

    pub fn build_double_click_cmd(&self, x: u32, y: u32) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                format!("cliclick dc:{x},{y}")
            }
            DesktopPlatform::Linux => {
                format!("xdotool mousemove {x} {y} click --repeat 2 --delay 50 1")
            }
            DesktopPlatform::Windows => {
                format!(
                    "powershell -command \"\
                    Add-Type -AssemblyName System.Windows.Forms; \
                    [System.Windows.Forms.Cursor]::Position = \
                    New-Object System.Drawing.Point({x},{y}); \
                    $sig = '[DllImport(\\\"user32.dll\\\")] public static extern void mouse_event(int f,int x,int y,int d,int e);'; \
                    $m = Add-Type -MemberDefinition $sig -Name M -Namespace W -PassThru; \
                    $m::mouse_event(0x0002,0,0,0,0); $m::mouse_event(0x0004,0,0,0,0); \
                    Start-Sleep -Milliseconds 50; \
                    $m::mouse_event(0x0002,0,0,0,0); $m::mouse_event(0x0004,0,0,0,0)\""
                )
            }
        }
    }

    async fn drag(
        &self,
        from_x: u32,
        from_y: u32,
        to_x: u32,
        to_y: u32,
    ) -> Result<DesktopResult> {
        let cmd = self.build_drag_cmd(from_x, from_y, to_x, to_y);
        run_shell(&cmd).await
    }

    pub fn build_drag_cmd(&self, from_x: u32, from_y: u32, to_x: u32, to_y: u32) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                format!("cliclick dd:{from_x},{from_y} du:{to_x},{to_y}")
            }
            DesktopPlatform::Linux => {
                format!(
                    "xdotool mousemove {from_x} {from_y} mousedown 1 \
                     mousemove {to_x} {to_y} mouseup 1"
                )
            }
            DesktopPlatform::Windows => {
                format!(
                    "powershell -command \"\
                    Add-Type -AssemblyName System.Windows.Forms; \
                    [System.Windows.Forms.Cursor]::Position = \
                    New-Object System.Drawing.Point({from_x},{from_y}); \
                    $sig = '[DllImport(\\\"user32.dll\\\")] public static extern void mouse_event(int f,int x,int y,int d,int e);'; \
                    $m = Add-Type -MemberDefinition $sig -Name M -Namespace W -PassThru; \
                    $m::mouse_event(0x0002,0,0,0,0); \
                    [System.Windows.Forms.Cursor]::Position = \
                    New-Object System.Drawing.Point({to_x},{to_y}); \
                    $m::mouse_event(0x0004,0,0,0,0)\""
                )
            }
        }
    }

    // ── Keyboard ────────────────────────────────────────────────────────

    async fn type_text(&self, text: &str) -> Result<DesktopResult> {
        let cmd = self.build_type_text_cmd(text);
        run_shell(&cmd).await
    }

    pub fn build_type_text_cmd(&self, text: &str) -> String {
        let escaped = shell_escape(text);
        match self.platform {
            DesktopPlatform::MacOS => {
                format!(
                    "osascript -e 'tell application \"System Events\" to keystroke \"{escaped}\"'"
                )
            }
            DesktopPlatform::Linux => {
                format!("xdotool type --delay 12 '{escaped}'")
            }
            DesktopPlatform::Windows => {
                let ps_escaped = text.replace('\'', "''").replace('{', "{{").replace('}', "}}");
                format!(
                    "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; \
                     [System.Windows.Forms.SendKeys]::SendWait('{ps_escaped}')\""
                )
            }
        }
    }

    async fn press_key(&self, key: &str) -> Result<DesktopResult> {
        let cmd = self.build_press_key_cmd(key);
        run_shell(&cmd).await
    }

    pub fn build_press_key_cmd(&self, key: &str) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                let mapped = key_name_to_osascript(key);
                format!(
                    "osascript -e 'tell application \"System Events\" to key code (key code \"{mapped}\")'"
                )
            }
            DesktopPlatform::Linux => {
                let mapped = key_name_to_xdotool(key);
                format!("xdotool key {mapped}")
            }
            DesktopPlatform::Windows => {
                let mapped = key_name_to_powershell(key);
                format!(
                    "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; \
                     [System.Windows.Forms.SendKeys]::SendWait('{mapped}')\""
                )
            }
        }
    }

    async fn key_combo(&self, modifiers: &[String], key: &str) -> Result<DesktopResult> {
        let cmd = self.build_key_combo_cmd(modifiers, key);
        run_shell(&cmd).await
    }

    pub fn build_key_combo_cmd(&self, modifiers: &[String], key: &str) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                let using_clause = modifiers
                    .iter()
                    .map(|m| format!("{} down", modifier_to_osascript(m)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let escaped_key = shell_escape(key);
                format!(
                    "osascript -e 'tell application \"System Events\" to keystroke \
                     \"{escaped_key}\" using {{{using_clause}}}'"
                )
            }
            DesktopPlatform::Linux => {
                let mapped_key = key_name_to_xdotool(key);
                let mod_prefix = modifiers
                    .iter()
                    .map(|m| modifier_to_xdotool(m).to_string())
                    .collect::<Vec<_>>()
                    .join("+");
                format!("xdotool key {mod_prefix}+{mapped_key}")
            }
            DesktopPlatform::Windows => {
                let ps_mods: String = modifiers
                    .iter()
                    .map(|m| modifier_to_powershell(m).to_string())
                    .collect();
                let mapped = key_name_to_powershell(key);
                format!(
                    "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; \
                     [System.Windows.Forms.SendKeys]::SendWait('{ps_mods}{mapped}')\""
                )
            }
        }
    }

    // ── Screenshot ──────────────────────────────────────────────────────

    async fn screenshot(&self, path: &str) -> Result<DesktopResult> {
        let cmd = self.build_screenshot_cmd(path);
        run_shell(&cmd).await
    }

    pub fn build_screenshot_cmd(&self, path: &str) -> String {
        let escaped = shell_escape(path);
        match self.platform {
            DesktopPlatform::MacOS => {
                format!("screencapture -x '{escaped}'")
            }
            DesktopPlatform::Linux => {
                format!("scrot '{escaped}'")
            }
            DesktopPlatform::Windows => {
                let ps_path = path.replace('\'', "''");
                format!(
                    "powershell -command \"\
                    Add-Type -AssemblyName System.Windows.Forms; \
                    Add-Type -AssemblyName System.Drawing; \
                    $s = [System.Windows.Forms.Screen]::PrimaryScreen; \
                    $b = New-Object System.Drawing.Bitmap($s.Bounds.Width,$s.Bounds.Height); \
                    $g = [System.Drawing.Graphics]::FromImage($b); \
                    $g.CopyFromScreen($s.Bounds.Location,[System.Drawing.Point]::Empty,$s.Bounds.Size); \
                    $b.Save('{ps_path}')\""
                )
            }
        }
    }

    // ── Window Management ───────────────────────────────────────────────

    async fn get_active_window(&self) -> Result<DesktopResult> {
        let cmd = self.build_get_active_window_cmd();
        let result = run_shell(&cmd).await?;
        if result.success {
            let windows = parse_window_list(&result.output, self.platform);
            let mut r = result;
            // Mark the first (only) window as focused
            let focused_wins: Vec<WindowInfo> = windows
                .into_iter()
                .map(|mut w| {
                    w.focused = true;
                    w
                })
                .collect();
            r.windows = Some(focused_wins);
            Ok(r)
        } else {
            Ok(result)
        }
    }

    pub fn build_get_active_window_cmd(&self) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                "osascript -e 'tell application \"System Events\" to get name of first process \
                 whose frontmost is true'"
                    .to_string()
            }
            DesktopPlatform::Linux => "xdotool getactivewindow getwindowname".to_string(),
            DesktopPlatform::Windows => {
                "powershell -command \"(Get-Process | Where-Object \
                 {$_.MainWindowHandle -eq (Add-Type -MemberDefinition \
                 '[DllImport(\\\"user32.dll\\\")] public static extern IntPtr \
                 GetForegroundWindow();' -Name W -Namespace U -PassThru)::GetForegroundWindow()}).MainWindowTitle\""
                    .to_string()
            }
        }
    }

    async fn focus_window(&self, title_pattern: &str) -> Result<DesktopResult> {
        let cmd = self.build_focus_window_cmd(title_pattern);
        run_shell(&cmd).await
    }

    pub fn build_focus_window_cmd(&self, title_pattern: &str) -> String {
        let escaped = shell_escape(title_pattern);
        match self.platform {
            DesktopPlatform::MacOS => {
                // Try to activate the application by name
                format!(
                    "osascript -e 'tell application \"{escaped}\" to activate'"
                )
            }
            DesktopPlatform::Linux => {
                format!("xdotool search --name '{escaped}' windowactivate")
            }
            DesktopPlatform::Windows => {
                let ps_escaped = title_pattern.replace('\'', "''");
                format!(
                    "powershell -command \"(Get-Process | Where-Object \
                     {{$_.MainWindowTitle -like '*{ps_escaped}*'}}).MainWindowHandle | \
                     ForEach-Object {{ \
                     Add-Type -MemberDefinition '[DllImport(\\\"user32.dll\\\")] public static extern bool SetForegroundWindow(IntPtr h);' \
                     -Name W -Namespace U -PassThru; [U.W]::SetForegroundWindow($_) }}\""
                )
            }
        }
    }

    async fn list_windows(&self) -> Result<DesktopResult> {
        let cmd = self.build_list_windows_cmd();
        let result = run_shell(&cmd).await?;
        if result.success {
            let windows = parse_window_list(&result.output, self.platform);
            let mut r = result;
            r.windows = Some(windows);
            Ok(r)
        } else {
            Ok(result)
        }
    }

    pub fn build_list_windows_cmd(&self) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                "osascript -e 'tell application \"System Events\" to \
                 repeat with p in (every process whose visible is true)\n\
                 set n to name of p\n\
                 repeat with w in (every window of p)\n\
                 log n & \" | \" & name of w\n\
                 end repeat\n\
                 end repeat'"
                    .to_string()
            }
            DesktopPlatform::Linux => "wmctrl -l".to_string(),
            DesktopPlatform::Windows => {
                "powershell -command \"Get-Process | Where-Object {$_.MainWindowTitle -ne ''} | \
                 ForEach-Object { $_.ProcessName + ' | ' + $_.MainWindowTitle + ' | ' + $_.Id }\""
                    .to_string()
            }
        }
    }

    async fn set_window_size(&self, width: u32, height: u32) -> Result<DesktopResult> {
        let cmd = self.build_set_window_size_cmd(width, height);
        run_shell(&cmd).await
    }

    pub fn build_set_window_size_cmd(&self, width: u32, height: u32) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                format!(
                    "osascript -e 'tell application \"System Events\" to tell (first process \
                     whose frontmost is true) to set size of front window to {{{width}, {height}}}'"
                )
            }
            DesktopPlatform::Linux => {
                format!(
                    "xdotool getactivewindow windowsize {width} {height}"
                )
            }
            DesktopPlatform::Windows => {
                format!(
                    "powershell -command \"\
                    $sig = '[DllImport(\\\"user32.dll\\\")] public static extern IntPtr GetForegroundWindow(); \
                    [DllImport(\\\"user32.dll\\\")] public static extern bool MoveWindow(IntPtr h,int x,int y,int w,int h2,bool r);'; \
                    $w = Add-Type -MemberDefinition $sig -Name W -Namespace U -PassThru; \
                    $h = $w::GetForegroundWindow(); \
                    $w::MoveWindow($h,0,0,{width},{height},$true)\""
                )
            }
        }
    }

    // ── Screen / Mouse Info ─────────────────────────────────────────────

    async fn get_screen_size(&self) -> Result<DesktopResult> {
        let cmd = self.build_get_screen_size_cmd();
        let result = run_shell(&cmd).await?;
        if result.success {
            let screen = parse_screen_size(&result.output, self.platform);
            let mut r = result;
            r.screen = screen;
            Ok(r)
        } else {
            Ok(result)
        }
    }

    pub fn build_get_screen_size_cmd(&self) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                "system_profiler SPDisplaysDataType | grep Resolution".to_string()
            }
            DesktopPlatform::Linux => "xdpyinfo | grep dimensions".to_string(),
            DesktopPlatform::Windows => {
                "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; \
                 $s = [System.Windows.Forms.Screen]::PrimaryScreen; \
                 Write-Output ('' + $s.Bounds.Width + '|' + $s.Bounds.Height)\""
                    .to_string()
            }
        }
    }

    async fn get_mouse_position(&self) -> Result<DesktopResult> {
        let cmd = self.build_get_mouse_position_cmd();
        let result = run_shell(&cmd).await?;
        if result.success {
            let pos = parse_mouse_position(&result.output, self.platform);
            let mut r = result;
            r.mouse_pos = pos;
            Ok(r)
        } else {
            Ok(result)
        }
    }

    pub fn build_get_mouse_position_cmd(&self) -> String {
        match self.platform {
            DesktopPlatform::MacOS => {
                "osascript -e 'tell application \"System Events\" to \
                 return position of the mouse'"
                    .to_string()
            }
            DesktopPlatform::Linux => "xdotool getmouselocation".to_string(),
            DesktopPlatform::Windows => {
                "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; \
                 $p = [System.Windows.Forms.Cursor]::Position; \
                 Write-Output ('' + $p.X + '|' + $p.Y)\""
                    .to_string()
            }
        }
    }
}

// ── Tool Availability ───────────────────────────────────────────────────────

/// Check which required CLI tools are missing for the current platform.
pub async fn check_prerequisites() -> Vec<String> {
    let platform = detect_platform();
    let tools: &[&str] = match platform {
        DesktopPlatform::MacOS => &["osascript", "screencapture", "cliclick"],
        DesktopPlatform::Linux => &["xdotool", "wmctrl", "scrot", "xdpyinfo"],
        DesktopPlatform::Windows => &["powershell"],
    };

    let mut missing = Vec::new();
    for tool in tools {
        let which_cmd = if cfg!(target_os = "windows") {
            format!("where {tool}")
        } else {
            format!("which {tool}")
        };
        let output = tokio::process::Command::new("sh")
            .args(["-c", &which_cmd])
            .output()
            .await;
        match output {
            Ok(o) if o.status.success() => {}
            _ => {
                missing.push(tool.to_string());
            }
        }
    }
    if !missing.is_empty() {
        warn!("Missing desktop automation tools: {missing:?}");
    }
    missing
}

/// Quick synchronous check whether the platform's primary automation tool
/// is on `$PATH`.
pub fn is_available() -> bool {
    let tool = match detect_platform() {
        DesktopPlatform::MacOS => "osascript",
        DesktopPlatform::Linux => "xdotool",
        DesktopPlatform::Windows => "powershell",
    };
    let which = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    std::process::Command::new(which)
        .arg(tool)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ── Internal Helpers ────────────────────────────────────────────────────────

/// Map a key name to PowerShell SendKeys syntax.
fn key_name_to_powershell(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "return" | "enter" => "{ENTER}".to_string(),
        "escape" | "esc" => "{ESC}".to_string(),
        "tab" => "{TAB}".to_string(),
        "space" => " ".to_string(),
        "delete" | "backspace" => "{BACKSPACE}".to_string(),
        "forwarddelete" | "del" => "{DELETE}".to_string(),
        "up" | "uparrow" => "{UP}".to_string(),
        "down" | "downarrow" => "{DOWN}".to_string(),
        "left" | "leftarrow" => "{LEFT}".to_string(),
        "right" | "rightarrow" => "{RIGHT}".to_string(),
        "home" => "{HOME}".to_string(),
        "end" => "{END}".to_string(),
        "pageup" => "{PGUP}".to_string(),
        "pagedown" => "{PGDN}".to_string(),
        "f1" => "{F1}".to_string(),
        "f2" => "{F2}".to_string(),
        "f3" => "{F3}".to_string(),
        "f4" => "{F4}".to_string(),
        "f5" => "{F5}".to_string(),
        "f6" => "{F6}".to_string(),
        "f7" => "{F7}".to_string(),
        "f8" => "{F8}".to_string(),
        "f9" => "{F9}".to_string(),
        "f10" => "{F10}".to_string(),
        "f11" => "{F11}".to_string(),
        "f12" => "{F12}".to_string(),
        other => other.to_string(),
    }
}

/// Execute a shell command and return a `DesktopResult`.
async fn run_shell(cmd: &str) -> Result<DesktopResult> {
    debug!("Running shell command: {cmd}");
    let output = tokio::process::Command::new("sh")
        .args(["-c", cmd])
        .output()
        .await
        .context("Failed to spawn shell command")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        let combined = if stderr.is_empty() {
            stdout
        } else {
            // Some tools (e.g., osascript `log`) write to stderr
            format!("{stdout}{stderr}")
        };
        Ok(DesktopResult::ok(combined.trim()))
    } else {
        let msg = if stderr.is_empty() {
            stdout
        } else {
            stderr
        };
        warn!("Shell command failed: {msg}");
        Ok(DesktopResult::fail(msg.trim()))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Platform detection ──────────────────────────────────────────────

    #[test]
    fn test_detect_platform_returns_valid() {
        let p = detect_platform();
        // On any OS, the result should be one of the three variants.
        matches!(
            p,
            DesktopPlatform::MacOS | DesktopPlatform::Linux | DesktopPlatform::Windows
        );
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(DesktopPlatform::MacOS.to_string(), "macOS");
        assert_eq!(DesktopPlatform::Linux.to_string(), "Linux");
        assert_eq!(DesktopPlatform::Windows.to_string(), "Windows");
    }

    #[test]
    fn test_platform_serde_roundtrip() {
        let p = DesktopPlatform::MacOS;
        let json = serde_json::to_string(&p).expect("serialize");
        let back: DesktopPlatform = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    // ── MouseButton ─────────────────────────────────────────────────────

    #[test]
    fn test_mouse_button_display() {
        assert_eq!(MouseButton::Left.to_string(), "left");
        assert_eq!(MouseButton::Right.to_string(), "right");
        assert_eq!(MouseButton::Middle.to_string(), "middle");
    }

    #[test]
    fn test_mouse_button_xdotool() {
        assert_eq!(MouseButton::Left.xdotool_button(), 1);
        assert_eq!(MouseButton::Right.xdotool_button(), 3);
        assert_eq!(MouseButton::Middle.xdotool_button(), 2);
    }

    #[test]
    fn test_mouse_button_serde_roundtrip() {
        for btn in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            let json = serde_json::to_string(&btn).expect("serialize");
            let back: MouseButton = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(btn, back);
        }
    }

    // ── DesktopAction serialization ─────────────────────────────────────

    #[test]
    fn test_action_move_mouse_serde() {
        let action = DesktopAction::MoveMouse { x: 100, y: 200 };
        let json = serde_json::to_string(&action).expect("serialize");
        assert!(json.contains("MoveMouse"));
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::MoveMouse { x, y } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_click_serde() {
        let action = DesktopAction::Click {
            button: MouseButton::Right,
            x: 50,
            y: 75,
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::Click { button, x, y } => {
                assert_eq!(button, MouseButton::Right);
                assert_eq!(x, 50);
                assert_eq!(y, 75);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_double_click_serde() {
        let action = DesktopAction::DoubleClick { x: 10, y: 20 };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::DoubleClick { x, y } => {
                assert_eq!(x, 10);
                assert_eq!(y, 20);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_drag_serde() {
        let action = DesktopAction::Drag {
            from_x: 0,
            from_y: 0,
            to_x: 500,
            to_y: 500,
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
            } => {
                assert_eq!(from_x, 0);
                assert_eq!(from_y, 0);
                assert_eq!(to_x, 500);
                assert_eq!(to_y, 500);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_type_text_serde() {
        let action = DesktopAction::TypeText {
            text: "Hello, World!".to_string(),
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::TypeText { text } => assert_eq!(text, "Hello, World!"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_press_key_serde() {
        let action = DesktopAction::PressKey {
            key: "Return".to_string(),
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::PressKey { key } => assert_eq!(key, "Return"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_key_combo_serde() {
        let action = DesktopAction::KeyCombo {
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            key: "t".to_string(),
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::KeyCombo { modifiers, key } => {
                assert_eq!(modifiers, vec!["ctrl", "shift"]);
                assert_eq!(key, "t");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_screenshot_serde() {
        let action = DesktopAction::Screenshot {
            path: "/tmp/shot.png".to_string(),
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::Screenshot { path } => assert_eq!(path, "/tmp/shot.png"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_get_active_window_serde() {
        let action = DesktopAction::GetActiveWindow;
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(back, DesktopAction::GetActiveWindow));
    }

    #[test]
    fn test_action_focus_window_serde() {
        let action = DesktopAction::FocusWindow {
            title_pattern: "Firefox".to_string(),
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::FocusWindow { title_pattern } => assert_eq!(title_pattern, "Firefox"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_list_windows_serde() {
        let json = serde_json::to_string(&DesktopAction::ListWindows).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(back, DesktopAction::ListWindows));
    }

    #[test]
    fn test_action_set_window_size_serde() {
        let action = DesktopAction::SetWindowSize {
            width: 1024,
            height: 768,
        };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::SetWindowSize { width, height } => {
                assert_eq!(width, 1024);
                assert_eq!(height, 768);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_action_delay_serde() {
        let action = DesktopAction::Delay { ms: 500 };
        let json = serde_json::to_string(&action).expect("serialize");
        let back: DesktopAction = serde_json::from_str(&json).expect("deserialize");
        match back {
            DesktopAction::Delay { ms } => assert_eq!(ms, 500),
            _ => panic!("wrong variant"),
        }
    }

    // ── Action Display ──────────────────────────────────────────────────

    #[test]
    fn test_action_display() {
        let a = DesktopAction::MoveMouse { x: 10, y: 20 };
        assert_eq!(a.to_string(), "MoveMouse(10, 20)");

        let a = DesktopAction::Click {
            button: MouseButton::Left,
            x: 5,
            y: 6,
        };
        assert_eq!(a.to_string(), "Click(left, 5, 6)");

        let a = DesktopAction::KeyCombo {
            modifiers: vec!["ctrl".into(), "alt".into()],
            key: "t".into(),
        };
        assert_eq!(a.to_string(), "KeyCombo(ctrl+alt+t)");
    }

    // ── WindowInfo ──────────────────────────────────────────────────────

    #[test]
    fn test_window_info_creation() {
        let w = WindowInfo {
            id: "0x1234".to_string(),
            title: "My Editor".to_string(),
            app: "code".to_string(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            focused: true,
        };
        assert_eq!(w.title, "My Editor");
        assert!(w.focused);
    }

    #[test]
    fn test_window_info_serde_roundtrip() {
        let w = WindowInfo {
            id: "42".to_string(),
            title: "Test".to_string(),
            app: "test_app".to_string(),
            x: -10,
            y: 20,
            width: 800,
            height: 600,
            focused: false,
        };
        let json = serde_json::to_string(&w).expect("serialize");
        let back: WindowInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, "42");
        assert_eq!(back.x, -10);
    }

    // ── ScreenInfo ──────────────────────────────────────────────────────

    #[test]
    fn test_screen_info_default() {
        let s = ScreenInfo::default();
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
        assert!((s.scale_factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_screen_info_serde() {
        let s = ScreenInfo {
            width: 2560,
            height: 1440,
            scale_factor: 2.0,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: ScreenInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.width, 2560);
        assert!((back.scale_factor - 2.0).abs() < f64::EPSILON);
    }

    // ── DesktopResult ───────────────────────────────────────────────────

    #[test]
    fn test_desktop_result_ok() {
        let r = DesktopResult::ok("done");
        assert!(r.success);
        assert_eq!(r.output, "done");
        assert!(r.windows.is_none());
        assert!(r.screen.is_none());
        assert!(r.mouse_pos.is_none());
    }

    #[test]
    fn test_desktop_result_fail() {
        let r = DesktopResult::fail("error occurred");
        assert!(!r.success);
        assert_eq!(r.output, "error occurred");
    }

    #[test]
    fn test_desktop_result_serde() {
        let mut r = DesktopResult::ok("test");
        r.mouse_pos = Some((100, 200));
        let json = serde_json::to_string(&r).expect("serialize");
        let back: DesktopResult = serde_json::from_str(&json).expect("deserialize");
        assert!(back.success);
        assert_eq!(back.mouse_pos, Some((100, 200)));
    }

    // ── Shell escape ────────────────────────────────────────────────────

    #[test]
    fn test_shell_escape_no_special_chars() {
        assert_eq!(shell_escape("hello"), "hello");
    }

    #[test]
    fn test_shell_escape_single_quote() {
        assert_eq!(shell_escape("it's"), "it'\\''s");
    }

    #[test]
    fn test_shell_escape_multiple_quotes() {
        assert_eq!(shell_escape("a'b'c"), "a'\\''b'\\''c");
    }

    #[test]
    fn test_shell_escape_spaces() {
        // Spaces don't need escaping inside single quotes
        assert_eq!(shell_escape("hello world"), "hello world");
    }

    #[test]
    fn test_shell_escape_empty() {
        assert_eq!(shell_escape(""), "");
    }

    #[test]
    fn test_shell_escape_special_chars() {
        // Dollar, backtick, backslash — safe inside single quotes
        assert_eq!(shell_escape("$HOME"), "$HOME");
        assert_eq!(shell_escape("`whoami`"), "`whoami`");
    }

    #[test]
    fn test_shell_escape_newlines() {
        assert_eq!(shell_escape("line1\nline2"), "line1\nline2");
    }

    // ── Key name mapping ────────────────────────────────────────────────

    #[test]
    fn test_key_name_to_osascript() {
        assert_eq!(key_name_to_osascript("Return"), "return");
        assert_eq!(key_name_to_osascript("enter"), "return");
        assert_eq!(key_name_to_osascript("Escape"), "escape");
        assert_eq!(key_name_to_osascript("Tab"), "tab");
        assert_eq!(key_name_to_osascript("space"), "space");
        assert_eq!(key_name_to_osascript("delete"), "delete");
        assert_eq!(key_name_to_osascript("up"), "up arrow");
        assert_eq!(key_name_to_osascript("down"), "down arrow");
        assert_eq!(key_name_to_osascript("pageup"), "page up");
        assert_eq!(key_name_to_osascript("F1"), "F1");
        assert_eq!(key_name_to_osascript("F12"), "F12");
        assert_eq!(key_name_to_osascript("a"), "a");
    }

    #[test]
    fn test_key_name_to_xdotool() {
        assert_eq!(key_name_to_xdotool("Return"), "Return");
        assert_eq!(key_name_to_xdotool("enter"), "Return");
        assert_eq!(key_name_to_xdotool("Escape"), "Escape");
        assert_eq!(key_name_to_xdotool("Tab"), "Tab");
        assert_eq!(key_name_to_xdotool("backspace"), "BackSpace");
        assert_eq!(key_name_to_xdotool("up"), "Up");
        assert_eq!(key_name_to_xdotool("pageup"), "Prior");
        assert_eq!(key_name_to_xdotool("pagedown"), "Next");
        assert_eq!(key_name_to_xdotool("F5"), "F5");
        assert_eq!(key_name_to_xdotool("z"), "z");
    }

    #[test]
    fn test_key_name_to_powershell() {
        assert_eq!(key_name_to_powershell("Return"), "{ENTER}");
        assert_eq!(key_name_to_powershell("Escape"), "{ESC}");
        assert_eq!(key_name_to_powershell("Tab"), "{TAB}");
        assert_eq!(key_name_to_powershell("backspace"), "{BACKSPACE}");
        assert_eq!(key_name_to_powershell("F1"), "{F1}");
        assert_eq!(key_name_to_powershell("pageup"), "{PGUP}");
    }

    // ── Modifier mapping ────────────────────────────────────────────────

    #[test]
    fn test_modifier_to_osascript() {
        assert_eq!(modifier_to_osascript("ctrl"), "control");
        assert_eq!(modifier_to_osascript("control"), "control");
        assert_eq!(modifier_to_osascript("alt"), "option");
        assert_eq!(modifier_to_osascript("option"), "option");
        assert_eq!(modifier_to_osascript("cmd"), "command");
        assert_eq!(modifier_to_osascript("command"), "command");
        assert_eq!(modifier_to_osascript("super"), "command");
        assert_eq!(modifier_to_osascript("meta"), "command");
        assert_eq!(modifier_to_osascript("shift"), "shift");
    }

    #[test]
    fn test_modifier_to_xdotool() {
        assert_eq!(modifier_to_xdotool("ctrl"), "ctrl");
        assert_eq!(modifier_to_xdotool("alt"), "alt");
        assert_eq!(modifier_to_xdotool("cmd"), "super");
        assert_eq!(modifier_to_xdotool("shift"), "shift");
    }

    #[test]
    fn test_modifier_to_powershell() {
        assert_eq!(modifier_to_powershell("ctrl"), "^");
        assert_eq!(modifier_to_powershell("alt"), "%");
        assert_eq!(modifier_to_powershell("shift"), "+");
        assert_eq!(modifier_to_powershell("cmd"), "^");
    }

    // ── Window list parsing ─────────────────────────────────────────────

    #[test]
    fn test_parse_window_list_macos() {
        let output = "Finder | Desktop | 1\nCode | main.rs — VibeCody | 42\n";
        let windows = parse_window_list(output, DesktopPlatform::MacOS);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].app, "Finder");
        assert_eq!(windows[0].title, "Desktop");
        assert_eq!(windows[0].id, "1");
        assert_eq!(windows[1].app, "Code");
        assert_eq!(windows[1].title, "main.rs — VibeCody");
        assert_eq!(windows[1].id, "42");
    }

    #[test]
    fn test_parse_window_list_macos_empty() {
        let windows = parse_window_list("", DesktopPlatform::MacOS);
        assert!(windows.is_empty());
    }

    #[test]
    fn test_parse_window_list_macos_missing_value() {
        let output = "missing value\nFinder | Desktop\n";
        let windows = parse_window_list(output, DesktopPlatform::MacOS);
        assert_eq!(windows.len(), 1);
    }

    #[test]
    fn test_parse_window_list_linux() {
        let output = "0x04000003  0 myhost Terminal\n0x04000007  0 myhost Firefox - Mozilla\n";
        let windows = parse_window_list(output, DesktopPlatform::Linux);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].id, "0x04000003");
        assert_eq!(windows[0].app, "myhost");
        assert_eq!(windows[0].title, "Terminal");
        assert_eq!(windows[1].title, "Firefox - Mozilla");
    }

    #[test]
    fn test_parse_window_list_linux_empty() {
        let windows = parse_window_list("", DesktopPlatform::Linux);
        assert!(windows.is_empty());
    }

    #[test]
    fn test_parse_window_list_windows() {
        let output = "chrome | Google - Chrome | 1234\nnotepad | Untitled - Notepad | 5678\n";
        let windows = parse_window_list(output, DesktopPlatform::Windows);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].app, "chrome");
        assert_eq!(windows[0].title, "Google - Chrome");
    }

    // ── Screen size parsing ─────────────────────────────────────────────

    #[test]
    fn test_parse_screen_size_macos_system_profiler() {
        let output = "          Resolution: 2560 x 1440 Retina\n";
        let screen = parse_screen_size(output, DesktopPlatform::MacOS);
        assert!(screen.is_some());
        let s = screen.unwrap();
        assert_eq!(s.width, 2560);
        assert_eq!(s.height, 1440);
        assert!((s.scale_factor - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_screen_size_macos_non_retina() {
        let output = "          Resolution: 1920 x 1080\n";
        let screen = parse_screen_size(output, DesktopPlatform::MacOS);
        assert!(screen.is_some());
        let s = screen.unwrap();
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
        assert!((s.scale_factor - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_screen_size_macos_bounds() {
        let output = "0, 0, 1440, 900";
        let screen = parse_screen_size(output, DesktopPlatform::MacOS);
        assert!(screen.is_some());
        let s = screen.unwrap();
        assert_eq!(s.width, 1440);
        assert_eq!(s.height, 900);
    }

    #[test]
    fn test_parse_screen_size_linux() {
        let output = "  dimensions:    1920x1080 pixels (508x285 millimeters)\n";
        let screen = parse_screen_size(output, DesktopPlatform::Linux);
        assert!(screen.is_some());
        let s = screen.unwrap();
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
    }

    #[test]
    fn test_parse_screen_size_linux_4k() {
        let output = "  dimensions:    3840x2160 pixels (600x340 millimeters)\n";
        let screen = parse_screen_size(output, DesktopPlatform::Linux);
        assert!(screen.is_some());
        let s = screen.unwrap();
        assert_eq!(s.width, 3840);
        assert_eq!(s.height, 2160);
    }

    #[test]
    fn test_parse_screen_size_windows() {
        let output = "1920|1080\n";
        let screen = parse_screen_size(output, DesktopPlatform::Windows);
        assert!(screen.is_some());
        let s = screen.unwrap();
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
    }

    #[test]
    fn test_parse_screen_size_invalid() {
        assert!(parse_screen_size("garbage", DesktopPlatform::MacOS).is_none());
        assert!(parse_screen_size("garbage", DesktopPlatform::Linux).is_none());
        assert!(parse_screen_size("garbage", DesktopPlatform::Windows).is_none());
    }

    // ── Mouse position parsing ──────────────────────────────────────────

    #[test]
    fn test_parse_mouse_position_macos() {
        let output = "{512, 384}\n";
        let pos = parse_mouse_position(output, DesktopPlatform::MacOS);
        assert_eq!(pos, Some((512, 384)));
    }

    #[test]
    fn test_parse_mouse_position_linux() {
        let output = "x:1234 y:567 screen:0 window:12345678\n";
        let pos = parse_mouse_position(output, DesktopPlatform::Linux);
        assert_eq!(pos, Some((1234, 567)));
    }

    #[test]
    fn test_parse_mouse_position_windows() {
        let output = "960|540\n";
        let pos = parse_mouse_position(output, DesktopPlatform::Windows);
        assert_eq!(pos, Some((960, 540)));
    }

    #[test]
    fn test_parse_mouse_position_invalid() {
        assert!(parse_mouse_position("garbage", DesktopPlatform::MacOS).is_none());
        assert!(parse_mouse_position("garbage", DesktopPlatform::Linux).is_none());
        assert!(parse_mouse_position("garbage", DesktopPlatform::Windows).is_none());
    }

    // ── Command construction (no execution) ─────────────────────────────

    #[test]
    fn test_build_move_mouse_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_move_mouse_cmd(100, 200);
        assert_eq!(cmd, "cliclick m:100,200");
    }

    #[test]
    fn test_build_move_mouse_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_move_mouse_cmd(100, 200);
        assert_eq!(cmd, "xdotool mousemove 100 200");
    }

    #[test]
    fn test_build_move_mouse_cmd_windows() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Windows);
        let cmd = da.build_move_mouse_cmd(100, 200);
        assert!(cmd.contains("System.Windows.Forms.Cursor"));
        assert!(cmd.contains("100,200"));
    }

    #[test]
    fn test_build_click_cmd_macos_left() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_click_cmd(MouseButton::Left, 50, 75);
        assert_eq!(cmd, "cliclick c:50,75");
    }

    #[test]
    fn test_build_click_cmd_macos_right() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_click_cmd(MouseButton::Right, 50, 75);
        assert_eq!(cmd, "cliclick rc:50,75");
    }

    #[test]
    fn test_build_click_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_click_cmd(MouseButton::Left, 50, 75);
        assert_eq!(cmd, "xdotool mousemove 50 75 click 1");
    }

    #[test]
    fn test_build_click_cmd_linux_right() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_click_cmd(MouseButton::Right, 50, 75);
        assert_eq!(cmd, "xdotool mousemove 50 75 click 3");
    }

    #[test]
    fn test_build_click_cmd_linux_middle() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_click_cmd(MouseButton::Middle, 50, 75);
        assert_eq!(cmd, "xdotool mousemove 50 75 click 2");
    }

    #[test]
    fn test_build_double_click_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_double_click_cmd(100, 200);
        assert_eq!(cmd, "cliclick dc:100,200");
    }

    #[test]
    fn test_build_double_click_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_double_click_cmd(100, 200);
        assert!(cmd.contains("--repeat 2"));
        assert!(cmd.contains("xdotool mousemove 100 200"));
    }

    #[test]
    fn test_build_drag_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_drag_cmd(10, 20, 300, 400);
        assert_eq!(cmd, "cliclick dd:10,20 du:300,400");
    }

    #[test]
    fn test_build_drag_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_drag_cmd(10, 20, 300, 400);
        assert!(cmd.contains("mousedown 1"));
        assert!(cmd.contains("mouseup 1"));
        assert!(cmd.contains("mousemove 10 20"));
        assert!(cmd.contains("mousemove 300 400"));
    }

    #[test]
    fn test_build_type_text_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_type_text_cmd("hello");
        assert!(cmd.contains("osascript"));
        assert!(cmd.contains("keystroke"));
        assert!(cmd.contains("hello"));
    }

    #[test]
    fn test_build_type_text_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_type_text_cmd("hello");
        assert_eq!(cmd, "xdotool type --delay 12 'hello'");
    }

    #[test]
    fn test_build_type_text_cmd_with_quotes() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_type_text_cmd("it's");
        assert!(cmd.contains("it'\\''s"));
    }

    #[test]
    fn test_build_press_key_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        assert_eq!(da.build_press_key_cmd("Return"), "xdotool key Return");
        assert_eq!(da.build_press_key_cmd("Escape"), "xdotool key Escape");
        assert_eq!(da.build_press_key_cmd("Tab"), "xdotool key Tab");
    }

    #[test]
    fn test_build_key_combo_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_key_combo_cmd(&["cmd".to_string()], "c");
        assert!(cmd.contains("command down"));
        assert!(cmd.contains("keystroke"));
    }

    #[test]
    fn test_build_key_combo_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_key_combo_cmd(&["ctrl".to_string()], "c");
        assert_eq!(cmd, "xdotool key ctrl+c");
    }

    #[test]
    fn test_build_key_combo_cmd_linux_multi_modifier() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd =
            da.build_key_combo_cmd(&["ctrl".to_string(), "shift".to_string()], "t");
        assert_eq!(cmd, "xdotool key ctrl+shift+t");
    }

    #[test]
    fn test_build_screenshot_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_screenshot_cmd("/tmp/shot.png");
        assert_eq!(cmd, "screencapture -x '/tmp/shot.png'");
    }

    #[test]
    fn test_build_screenshot_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_screenshot_cmd("/tmp/shot.png");
        assert_eq!(cmd, "scrot '/tmp/shot.png'");
    }

    #[test]
    fn test_build_list_windows_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        assert_eq!(da.build_list_windows_cmd(), "wmctrl -l");
    }

    #[test]
    fn test_build_focus_window_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_focus_window_cmd("Firefox");
        assert_eq!(cmd, "xdotool search --name 'Firefox' windowactivate");
    }

    #[test]
    fn test_build_focus_window_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_focus_window_cmd("Safari");
        assert!(cmd.contains("osascript"));
        assert!(cmd.contains("Safari"));
        assert!(cmd.contains("activate"));
    }

    #[test]
    fn test_build_set_window_size_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_set_window_size_cmd(800, 600);
        assert_eq!(cmd, "xdotool getactivewindow windowsize 800 600");
    }

    #[test]
    fn test_build_get_screen_size_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let cmd = da.build_get_screen_size_cmd();
        assert_eq!(cmd, "xdpyinfo | grep dimensions");
    }

    #[test]
    fn test_build_get_screen_size_cmd_macos() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::MacOS);
        let cmd = da.build_get_screen_size_cmd();
        assert!(cmd.contains("system_profiler SPDisplaysDataType"));
    }

    #[test]
    fn test_build_get_mouse_position_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        assert_eq!(da.build_get_mouse_position_cmd(), "xdotool getmouselocation");
    }

    #[test]
    fn test_build_get_active_window_cmd_linux() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        assert_eq!(
            da.build_get_active_window_cmd(),
            "xdotool getactivewindow getwindowname"
        );
    }

    // ── Config defaults ─────────────────────────────────────────────────

    #[test]
    fn test_automation_default_delay() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        assert_eq!(da.action_delay_ms, 100);
    }

    #[test]
    fn test_automation_new_sets_platform() {
        let da = DesktopAutomation::new();
        // Should match the compile-time platform
        if cfg!(target_os = "macos") {
            assert_eq!(da.platform, DesktopPlatform::MacOS);
        } else if cfg!(target_os = "windows") {
            assert_eq!(da.platform, DesktopPlatform::Windows);
        } else {
            assert_eq!(da.platform, DesktopPlatform::Linux);
        }
    }

    // ── Destructive action classification ───────────────────────────────

    #[test]
    fn test_desktop_actions_are_reversible() {
        // Desktop actions are inherently reversible (move mouse back, undo typing, etc.)
        // so there is no "destructive" classification. This test documents that design.
        let actions = vec![
            DesktopAction::MoveMouse { x: 0, y: 0 },
            DesktopAction::Click {
                button: MouseButton::Left,
                x: 0,
                y: 0,
            },
            DesktopAction::TypeText {
                text: "a".to_string(),
            },
            DesktopAction::Screenshot {
                path: "/tmp/x.png".to_string(),
            },
            DesktopAction::GetActiveWindow,
            DesktopAction::ListWindows,
            DesktopAction::GetScreenSize,
            DesktopAction::GetMousePosition,
            DesktopAction::Delay { ms: 1 },
        ];
        // All actions are valid — no "destructive" subset
        assert_eq!(actions.len(), 9);
    }

    // ── Async integration-style tests ───────────────────────────────────

    #[tokio::test]
    async fn test_execute_delay_action() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let result = da.execute(&DesktopAction::Delay { ms: 10 }).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.success);
        assert!(r.output.contains("10ms"));
    }

    #[tokio::test]
    async fn test_execute_sequence_empty() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let results = da.execute_sequence(&[]).await;
        assert!(results.is_ok());
        assert!(results.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_execute_sequence_delays_only() {
        let da = DesktopAutomation::for_platform(DesktopPlatform::Linux);
        let actions = vec![
            DesktopAction::Delay { ms: 5 },
            DesktopAction::Delay { ms: 5 },
        ];
        let results = da.execute_sequence(&actions).await;
        assert!(results.is_ok());
        let r = results.unwrap();
        assert_eq!(r.len(), 2);
        assert!(r[0].success);
        assert!(r[1].success);
    }
}
