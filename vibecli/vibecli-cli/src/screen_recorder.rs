#![allow(dead_code)]
//! Agent screen recording — captures screenshots during agent execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingFrame {
    pub path: String,
    pub timestamp: u64,
    pub caption: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub session_id: String,
    pub frames: Vec<RecordingFrame>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}

pub struct ScreenRecorder {
    session_id: String,
    output_dir: PathBuf,
    frames: Vec<RecordingFrame>,
    started_at: u64,
}

impl ScreenRecorder {
    pub fn new(session_id: &str) -> Result<Self> {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("recordings")
            .join(session_id);
        std::fs::create_dir_all(&dir)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Ok(Self {
            session_id: session_id.to_string(),
            output_dir: dir,
            frames: Vec::new(),
            started_at: now,
        })
    }

    /// Capture a screenshot with a descriptive caption.
    pub fn capture_frame(&mut self, caption: &str) -> Result<RecordingFrame> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let frame_path = self.output_dir.join(format!("frame-{:04}.png", self.frames.len()));

        take_screenshot_to(&frame_path)?;

        let frame = RecordingFrame {
            path: frame_path.to_string_lossy().to_string(),
            timestamp: ts,
            caption: caption.to_string(),
        };
        self.frames.push(frame.clone());
        Ok(frame)
    }

    /// Finish recording and save metadata.
    pub fn finish(&mut self) -> Result<Recording> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let recording = Recording {
            session_id: self.session_id.clone(),
            frames: self.frames.clone(),
            started_at: self.started_at,
            finished_at: Some(now),
        };
        let meta_path = self.output_dir.join("recording.json");
        std::fs::write(&meta_path, serde_json::to_string_pretty(&recording)?)?;
        Ok(recording)
    }

    /// List all saved recordings.
    pub fn list_recordings() -> Result<Vec<Recording>> {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("recordings");
        let mut recordings = Vec::new();
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let meta = entry.path().join("recording.json");
                if meta.exists() {
                    if let Ok(content) = std::fs::read_to_string(&meta) {
                        if let Ok(rec) = serde_json::from_str::<Recording>(&content) {
                            recordings.push(rec);
                        }
                    }
                }
            }
        }
        recordings.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(recordings)
    }
}

/// Platform-specific screenshot capture.
fn take_screenshot_to(output_path: &Path) -> Result<()> {
    let cmd = if cfg!(target_os = "macos") {
        format!("screencapture -x {}", output_path.display())
    } else if cfg!(target_os = "linux") {
        format!("scrot {}", output_path.display())
    } else {
        return Err(anyhow::anyhow!("Screenshot not supported on this platform"));
    };
    let output = std::process::Command::new("sh")
        .args(["-c", &cmd])
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Screenshot failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_serde() {
        let rec = Recording {
            session_id: "test-123".to_string(),
            frames: vec![RecordingFrame {
                path: "/tmp/frame.png".to_string(),
                timestamp: 1234567890,
                caption: "Initial state".to_string(),
            }],
            started_at: 1234567880,
            finished_at: Some(1234567900),
        };
        let json = serde_json::to_string(&rec).unwrap();
        let parsed: Recording = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.session_id, "test-123");
        assert_eq!(parsed.frames.len(), 1);
    }

    #[test]
    fn frame_serde() {
        let frame = RecordingFrame {
            path: "/tmp/test.png".to_string(),
            timestamp: 1000,
            caption: "Button clicked".to_string(),
        };
        let json = serde_json::to_string(&frame).unwrap();
        let parsed: RecordingFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.caption, "Button clicked");
    }

    #[test]
    fn list_recordings_empty() {
        // Should not crash even if directory doesn't exist
        let result = ScreenRecorder::list_recordings();
        assert!(result.is_ok());
    }

    #[test]
    fn recording_frame_fields() {
        let frame = RecordingFrame {
            path: "/tmp/frame-0000.png".to_string(),
            timestamp: 999,
            caption: "initial".to_string(),
        };
        assert_eq!(frame.path, "/tmp/frame-0000.png");
        assert_eq!(frame.timestamp, 999);
        assert_eq!(frame.caption, "initial");
    }

    #[test]
    fn recording_frame_clone() {
        let frame = RecordingFrame {
            path: "/a/b.png".to_string(),
            timestamp: 42,
            caption: "cloned".to_string(),
        };
        let cloned = frame.clone();
        assert_eq!(cloned.path, frame.path);
        assert_eq!(cloned.timestamp, frame.timestamp);
        assert_eq!(cloned.caption, frame.caption);
    }

    #[test]
    fn recording_no_finished_at() {
        let rec = Recording {
            session_id: "sess-1".to_string(),
            frames: vec![],
            started_at: 100,
            finished_at: None,
        };
        assert!(rec.finished_at.is_none());
        assert!(rec.frames.is_empty());
    }

    #[test]
    fn recording_with_finished_at() {
        let rec = Recording {
            session_id: "sess-2".to_string(),
            frames: vec![],
            started_at: 100,
            finished_at: Some(200),
        };
        assert_eq!(rec.finished_at, Some(200));
    }

    #[test]
    fn recording_multiple_frames() {
        let rec = Recording {
            session_id: "multi".to_string(),
            frames: vec![
                RecordingFrame { path: "a.png".into(), timestamp: 1, caption: "first".into() },
                RecordingFrame { path: "b.png".into(), timestamp: 2, caption: "second".into() },
                RecordingFrame { path: "c.png".into(), timestamp: 3, caption: "third".into() },
            ],
            started_at: 0,
            finished_at: Some(10),
        };
        assert_eq!(rec.frames.len(), 3);
        assert_eq!(rec.frames[0].caption, "first");
        assert_eq!(rec.frames[2].caption, "third");
    }

    #[test]
    fn recording_serde_roundtrip_no_frames() {
        let rec = Recording {
            session_id: "empty-rec".to_string(),
            frames: vec![],
            started_at: 50,
            finished_at: None,
        };
        let json = serde_json::to_string(&rec).unwrap();
        let parsed: Recording = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.session_id, "empty-rec");
        assert!(parsed.frames.is_empty());
        assert!(parsed.finished_at.is_none());
    }

    #[test]
    fn recording_frame_serde_roundtrip() {
        let frame = RecordingFrame {
            path: "/x/y/z.png".to_string(),
            timestamp: u64::MAX,
            caption: "max ts".to_string(),
        };
        let json = serde_json::to_string(&frame).unwrap();
        let parsed: RecordingFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.timestamp, u64::MAX);
    }

    #[test]
    fn recording_clone() {
        let rec = Recording {
            session_id: "clone-test".to_string(),
            frames: vec![RecordingFrame {
                path: "f.png".into(),
                timestamp: 5,
                caption: "cap".into(),
            }],
            started_at: 1,
            finished_at: Some(10),
        };
        let cloned = rec.clone();
        assert_eq!(cloned.session_id, rec.session_id);
        assert_eq!(cloned.frames.len(), rec.frames.len());
        assert_eq!(cloned.started_at, rec.started_at);
        assert_eq!(cloned.finished_at, rec.finished_at);
    }

    #[test]
    fn recording_debug_format() {
        let rec = Recording {
            session_id: "dbg".to_string(),
            frames: vec![],
            started_at: 0,
            finished_at: None,
        };
        let debug = format!("{:?}", rec);
        assert!(debug.contains("dbg"));
        assert!(debug.contains("Recording"));
    }

    #[test]
    fn recording_frame_debug_format() {
        let frame = RecordingFrame {
            path: "test.png".to_string(),
            timestamp: 123,
            caption: "debug".to_string(),
        };
        let debug = format!("{:?}", frame);
        assert!(debug.contains("RecordingFrame"));
        assert!(debug.contains("debug"));
    }

    #[test]
    fn recording_serde_pretty_json() {
        let rec = Recording {
            session_id: "pretty".to_string(),
            frames: vec![],
            started_at: 1000,
            finished_at: Some(2000),
        };
        let json = serde_json::to_string_pretty(&rec).unwrap();
        assert!(json.contains('\n'));
        let parsed: Recording = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.session_id, "pretty");
    }
}
