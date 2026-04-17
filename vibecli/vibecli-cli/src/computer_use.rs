/*!
 * computer_use.rs — Desktop automation action model.
 *
 * Represent, validate, and serialize desktop actions (click, type, screenshot,
 * scroll, key press). Stub execution for non-macOS environments.
 */

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Key {
    Enter,
    Escape,
    Tab,
    Backspace,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Click { x: i32, y: i32, button: MouseButton },
    DoubleClick { x: i32, y: i32 },
    Type { text: String },
    KeyPress { key: Key },
    Scroll { x: i32, y: i32, delta_x: i32, delta_y: i32 },
    Screenshot,
    MoveMouse { x: i32, y: i32 },
}

// ---------------------------------------------------------------------------
// Action helpers
// ---------------------------------------------------------------------------

impl Action {
    /// Returns true if the action mutates state (types text or presses Enter).
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            Action::Type { .. } | Action::KeyPress { key: Key::Enter }
        )
    }

    /// Human-readable description of the action.
    pub fn description(&self) -> String {
        match self {
            Action::Click { x, y, button } => {
                format!("Click {:?} at ({}, {})", button, x, y)
            }
            Action::DoubleClick { x, y } => format!("Double-click at ({}, {})", x, y),
            Action::Type { text } => format!("Type \"{}\"", text),
            Action::KeyPress { key } => format!("Press {:?}", key),
            Action::Scroll { x, y, delta_x, delta_y } => {
                format!("Scroll at ({}, {}) by ({}, {})", x, y, delta_x, delta_y)
            }
            Action::Screenshot => "Take screenshot".to_string(),
            Action::MoveMouse { x, y } => format!("Move mouse to ({}, {})", x, y),
        }
    }
}

// ---------------------------------------------------------------------------
// ScreenBounds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenBounds {
    pub width: u32,
    pub height: u32,
}

impl ScreenBounds {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn validate_action(&self, action: &Action) -> Result<(), String> {
        match action {
            Action::Click { x, y, .. } | Action::DoubleClick { x, y } => {
                if self.contains(*x, *y) {
                    Ok(())
                } else {
                    Err(format!(
                        "Coordinates ({}, {}) out of bounds {}x{}",
                        x, y, self.width, self.height
                    ))
                }
            }
            Action::Scroll { x, y, .. } => {
                if self.contains(*x, *y) {
                    Ok(())
                } else {
                    Err(format!(
                        "Scroll position ({}, {}) out of bounds {}x{}",
                        x, y, self.width, self.height
                    ))
                }
            }
            Action::MoveMouse { x, y } => {
                if self.contains(*x, *y) {
                    Ok(())
                } else {
                    Err(format!(
                        "Move target ({}, {}) out of bounds {}x{}",
                        x, y, self.width, self.height
                    ))
                }
            }
            _ => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// ActionResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ActionResult {
    pub success: bool,
    pub message: Option<String>,
    pub screenshot_b64: Option<String>,
}

impl ActionResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            message: None,
            screenshot_b64: None,
        }
    }

    pub fn fail(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(msg.into()),
            screenshot_b64: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ActionPlan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ActionPlan {
    pub goal: String,
    pub steps: Vec<Action>,
}

impl ActionPlan {
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            steps: Vec::new(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, action: Action) -> Self {
        self.steps.push(action);
        self
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn destructive_count(&self) -> usize {
        self.steps.iter().filter(|a| a.is_destructive()).count()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_description() {
        let a = Action::Click { x: 10, y: 20, button: MouseButton::Left };
        let desc = a.description();
        assert!(desc.contains("10"));
        assert!(desc.contains("20"));
        assert!(desc.contains("Left"));
    }

    #[test]
    fn test_type_is_destructive() {
        let a = Action::Type { text: "hello".into() };
        assert!(a.is_destructive());
    }

    #[test]
    fn test_screenshot_not_destructive() {
        assert!(!Action::Screenshot.is_destructive());
    }

    #[test]
    fn test_screen_bounds_contains_valid() {
        let bounds = ScreenBounds::new(1920, 1080);
        assert!(bounds.contains(0, 0));
        assert!(bounds.contains(1919, 1079));
        assert!(bounds.contains(960, 540));
    }

    #[test]
    fn test_screen_bounds_rejects_out_of_bounds() {
        let bounds = ScreenBounds::new(1920, 1080);
        assert!(!bounds.contains(-1, 0));
        assert!(!bounds.contains(1920, 0));
        assert!(!bounds.contains(0, 1080));
    }

    #[test]
    fn test_validate_action_ok_within_bounds() {
        let bounds = ScreenBounds::new(1920, 1080);
        let action = Action::Click { x: 100, y: 200, button: MouseButton::Left };
        assert!(bounds.validate_action(&action).is_ok());
    }

    #[test]
    fn test_validate_action_fails_out_of_bounds() {
        let bounds = ScreenBounds::new(800, 600);
        let action = Action::Click { x: 900, y: 700, button: MouseButton::Right };
        assert!(bounds.validate_action(&action).is_err());
    }

    #[test]
    fn test_action_plan_step_and_destructive_count() {
        let plan = ActionPlan::new("Fill form")
            .add(Action::Click { x: 10, y: 10, button: MouseButton::Left })
            .add(Action::Type { text: "Alice".into() })
            .add(Action::KeyPress { key: Key::Enter })
            .add(Action::Screenshot);
        assert_eq!(plan.step_count(), 4);
        assert_eq!(plan.destructive_count(), 2); // Type + KeyPress(Enter)
    }
}
