//! Energy voice-activity detection with hangover.
//!
//! Deliberately not a neural VAD. Silero v5 is the right production answer, but
//! a demo that ships its own 2 MB model hides the part worth showing — the
//! *turn* logic — behind a download. The thresholds below are the only tuning
//! that matters and they are named, not buried.

/// Frames are 20 ms at 16 kHz.
pub const FRAME: usize = 320;
const SPEECH_DBFS: f32 = -45.0;
/// Silence this long ends a turn. 600 ms is the figure the 2026 local-stack
/// write-ups converge on: shorter clips words, longer feels like lag.
const HANGOVER_MS: u32 = 600;
/// Ignore blips so a keyboard click cannot open a turn.
const MIN_SPEECH_MS: u32 = 200;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Turn {
    Silence,
    /// Speech began this frame — the barge-in signal.
    SpeechStart,
    Speech,
    /// Speech ended: the utterance is complete.
    SpeechEnd,
}

pub struct Vad {
    in_speech: bool,
    silence_ms: u32,
    speech_ms: u32,
}

impl Default for Vad {
    fn default() -> Self { Self { in_speech: false, silence_ms: 0, speech_ms: 0 } }
}

pub fn dbfs(frame: &[i16]) -> f32 {
    if frame.is_empty() { return -120.0; }
    let sum: f64 = frame.iter().map(|&s| { let f = s as f64 / 32768.0; f * f }).sum();
    let rms = (sum / frame.len() as f64).sqrt();
    if rms <= 1e-9 { -120.0 } else { 20.0 * rms.log10() as f32 }
}

impl Vad {
    /// Feed one 20 ms frame.
    pub fn push(&mut self, frame: &[i16]) -> Turn {
        let loud = dbfs(frame) > SPEECH_DBFS;
        if loud {
            self.silence_ms = 0;
            self.speech_ms += 20;
            if !self.in_speech && self.speech_ms >= MIN_SPEECH_MS {
                self.in_speech = true;
                return Turn::SpeechStart;
            }
            return if self.in_speech { Turn::Speech } else { Turn::Silence };
        }
        self.speech_ms = 0;
        if self.in_speech {
            self.silence_ms += 20;
            if self.silence_ms >= HANGOVER_MS {
                self.in_speech = false;
                self.silence_ms = 0;
                return Turn::SpeechEnd;
            }
            return Turn::Speech;
        }
        Turn::Silence
    }
}
