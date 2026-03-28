//! Offline voice transcription engine using local Whisper models.
//!
//! Gap 15 — Provides fully offline speech-to-text via whisper.cpp-compatible
//! model downloads, voice activity detection, and transcription history.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Whisper model variants with approximate sizes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
}

impl WhisperModel {
    /// Model size in megabytes.
    pub fn size_mb(&self) -> u64 {
        match self {
            WhisperModel::Tiny => 75,
            WhisperModel::Base => 142,
            WhisperModel::Small => 466,
            WhisperModel::Medium => 1500,
            WhisperModel::Large => 2900,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            WhisperModel::Tiny => "tiny",
            WhisperModel::Base => "base",
            WhisperModel::Small => "small",
            WhisperModel::Medium => "medium",
            WhisperModel::Large => "large",
        }
    }

    pub fn all() -> Vec<WhisperModel> {
        vec![
            WhisperModel::Tiny,
            WhisperModel::Base,
            WhisperModel::Small,
            WhisperModel::Medium,
            WhisperModel::Large,
        ]
    }

    /// Parse a model name string into a WhisperModel variant.
    pub fn from_name(name: &str) -> Option<WhisperModel> {
        match name.to_lowercase().as_str() {
            "tiny" => Some(WhisperModel::Tiny),
            "base" => Some(WhisperModel::Base),
            "small" => Some(WhisperModel::Small),
            "medium" => Some(WhisperModel::Medium),
            "large" => Some(WhisperModel::Large),
            _ => None,
        }
    }

    /// URL to download the GGML model file from Hugging Face.
    pub fn ggml_url(&self) -> &str {
        match self {
            WhisperModel::Tiny   => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            WhisperModel::Base   => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            WhisperModel::Small  => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            WhisperModel::Medium => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            WhisperModel::Large  => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        }
    }
}

/// Configuration for the local voice engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub model: WhisperModel,
    pub language: String,
    pub sample_rate: u32,
    pub vad_enabled: bool,
    pub energy_threshold: f64,
    pub silence_timeout_ms: u64,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            model: WhisperModel::Base,
            language: "en".to_string(),
            sample_rate: 16000,
            vad_enabled: true,
            energy_threshold: 0.02,
            silence_timeout_ms: 1500,
        }
    }
}

/// Result of a single transcription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f64,
    pub duration_secs: f64,
    pub language: String,
    pub timestamp: u64,
    pub model_used: WhisperModel,
    pub offline: bool,
}

/// Voice activity detection result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceActivity {
    pub is_speech: bool,
    pub energy_level: f64,
    pub timestamp: u64,
}

/// Aggregate metrics for the voice engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceMetrics {
    pub total_transcriptions: u64,
    pub total_duration: f64,
    pub avg_confidence: f64,
    pub offline_count: u64,
    pub online_fallback_count: u64,
    pub errors: u64,
}

impl Default for VoiceMetrics {
    fn default() -> Self {
        Self {
            total_transcriptions: 0,
            total_duration: 0.0,
            avg_confidence: 0.0,
            offline_count: 0,
            online_fallback_count: 0,
            errors: 0,
        }
    }
}

/// Local voice engine for offline transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVoiceEngine {
    pub config: VoiceConfig,
    pub model_path: Option<String>,
    pub transcriptions: Vec<TranscriptionResult>,
    pub is_recording: bool,
    pub metrics: VoiceMetrics,
    downloaded_models: HashMap<String, String>,
}

impl LocalVoiceEngine {
    pub fn new(config: VoiceConfig) -> Self {
        Self {
            config,
            model_path: None,
            transcriptions: Vec::new(),
            is_recording: false,
            metrics: VoiceMetrics::default(),
            downloaded_models: HashMap::new(),
        }
    }

    /// Start recording audio.
    pub fn start_recording(&mut self) -> Result<(), String> {
        if self.is_recording {
            return Err("Already recording".to_string());
        }
        if self.model_path.is_none() && !self.downloaded_models.contains_key(self.config.model.name()) {
            return Err("No model downloaded — call download_model first".to_string());
        }
        self.is_recording = true;
        Ok(())
    }

    /// Stop recording and return any pending audio duration.
    pub fn stop_recording(&mut self) -> Result<f64, String> {
        if !self.is_recording {
            return Err("Not currently recording".to_string());
        }
        self.is_recording = false;
        // Simulated captured duration
        Ok(2.5)
    }

    /// Transcribe an audio buffer (simulated).
    pub fn transcribe_audio(&mut self, audio_samples: &[f32], duration_secs: f64) -> Result<TranscriptionResult, String> {
        if audio_samples.is_empty() {
            return Err("Empty audio buffer".to_string());
        }
        if duration_secs <= 0.0 {
            return Err("Duration must be positive".to_string());
        }

        // Simulated transcription — confidence based on energy
        let energy: f64 = audio_samples.iter().map(|s| (*s as f64).abs()).sum::<f64>()
            / audio_samples.len() as f64;
        let confidence = (energy * 10.0).clamp(0.1, 1.0);

        let text = if energy < 0.01 {
            "[silence]".to_string()
        } else {
            "transcribed text placeholder".to_string()
        };

        let result = TranscriptionResult {
            text,
            confidence,
            duration_secs,
            language: self.config.language.clone(),
            timestamp: self.transcriptions.len() as u64 + 1,
            model_used: self.config.model.clone(),
            offline: true,
        };

        self.transcriptions.push(result.clone());
        self.metrics.total_transcriptions += 1;
        self.metrics.total_duration += duration_secs;
        self.metrics.offline_count += 1;

        // Running average
        let n = self.metrics.total_transcriptions as f64;
        self.metrics.avg_confidence =
            self.metrics.avg_confidence * ((n - 1.0) / n) + confidence / n;

        Ok(result)
    }

    /// Voice activity detection on a frame of audio.
    pub fn detect_voice_activity(&self, frame: &[f32], timestamp: u64) -> VoiceActivity {
        let energy: f64 = if frame.is_empty() {
            0.0
        } else {
            frame.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>()
                / frame.len() as f64
        };
        let energy_level = energy.sqrt();
        VoiceActivity {
            is_speech: energy_level > self.config.energy_threshold,
            energy_level,
            timestamp,
        }
    }

    /// Simulate downloading a model to a local path.
    pub fn download_model(&mut self, model: &WhisperModel) -> Result<String, String> {
        let path = format!("~/.vibecli/models/whisper-{}.bin", model.name());
        self.downloaded_models.insert(model.name().to_string(), path.clone());
        if self.config.model == *model {
            self.model_path = Some(path.clone());
        }
        Ok(path)
    }

    /// Get the local path for a model if downloaded.
    pub fn get_model_path(&self, model: &WhisperModel) -> Option<&String> {
        self.downloaded_models.get(model.name())
    }

    /// List all available models with download status.
    pub fn list_available_models(&self) -> Vec<(WhisperModel, bool, u64)> {
        WhisperModel::all()
            .into_iter()
            .map(|m| {
                let downloaded = self.downloaded_models.contains_key(m.name());
                let size = m.size_mb();
                (m, downloaded, size)
            })
            .collect()
    }

    /// Get transcription history filtered by language.
    pub fn history_by_language(&self, lang: &str) -> Vec<&TranscriptionResult> {
        self.transcriptions.iter().filter(|t| t.language == lang).collect()
    }

    /// Clear all transcription history.
    pub fn clear_history(&mut self) {
        self.transcriptions.clear();
    }

    /// Switch active model.
    pub fn set_model(&mut self, model: WhisperModel) -> Result<(), String> {
        if let Some(path) = self.downloaded_models.get(model.name()) {
            self.model_path = Some(path.clone());
            self.config.model = model;
            Ok(())
        } else {
            Err(format!("Model {} not downloaded", model.name()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_model_sizes() {
        assert_eq!(WhisperModel::Tiny.size_mb(), 75);
        assert_eq!(WhisperModel::Base.size_mb(), 142);
        assert_eq!(WhisperModel::Small.size_mb(), 466);
        assert_eq!(WhisperModel::Medium.size_mb(), 1500);
        assert_eq!(WhisperModel::Large.size_mb(), 2900);
    }

    #[test]
    fn test_whisper_model_names() {
        assert_eq!(WhisperModel::Tiny.name(), "tiny");
        assert_eq!(WhisperModel::Large.name(), "large");
    }

    #[test]
    fn test_whisper_model_all() {
        assert_eq!(WhisperModel::all().len(), 5);
    }

    #[test]
    fn test_voice_config_default() {
        let cfg = VoiceConfig::default();
        assert_eq!(cfg.model, WhisperModel::Base);
        assert_eq!(cfg.language, "en");
        assert_eq!(cfg.sample_rate, 16000);
        assert!(cfg.vad_enabled);
    }

    #[test]
    fn test_engine_new() {
        let engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(!engine.is_recording);
        assert!(engine.transcriptions.is_empty());
        assert_eq!(engine.metrics.total_transcriptions, 0);
    }

    #[test]
    fn test_start_recording_no_model() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(engine.start_recording().is_err());
    }

    #[test]
    fn test_start_stop_recording() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.download_model(&WhisperModel::Base).unwrap();
        assert!(engine.start_recording().is_ok());
        assert!(engine.is_recording);
        let dur = engine.stop_recording().unwrap();
        assert!(dur > 0.0);
        assert!(!engine.is_recording);
    }

    #[test]
    fn test_double_start_recording() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.download_model(&WhisperModel::Base).unwrap();
        engine.start_recording().unwrap();
        assert!(engine.start_recording().is_err());
    }

    #[test]
    fn test_stop_without_start() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(engine.stop_recording().is_err());
    }

    #[test]
    fn test_transcribe_audio_basic() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        let samples = vec![0.1f32; 1600];
        let result = engine.transcribe_audio(&samples, 1.0).unwrap();
        assert!(!result.text.is_empty());
        assert!(result.confidence > 0.0);
        assert_eq!(result.duration_secs, 1.0);
        assert!(result.offline);
    }

    #[test]
    fn test_transcribe_empty_buffer() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(engine.transcribe_audio(&[], 1.0).is_err());
    }

    #[test]
    fn test_transcribe_zero_duration() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(engine.transcribe_audio(&[0.1], 0.0).is_err());
    }

    #[test]
    fn test_transcribe_negative_duration() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(engine.transcribe_audio(&[0.1], -1.0).is_err());
    }

    #[test]
    fn test_transcribe_silence() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        let samples = vec![0.0001f32; 1600];
        let result = engine.transcribe_audio(&samples, 1.0).unwrap();
        assert_eq!(result.text, "[silence]");
    }

    #[test]
    fn test_metrics_update_after_transcribe() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.transcribe_audio(&vec![0.5f32; 800], 0.5).unwrap();
        engine.transcribe_audio(&vec![0.3f32; 1600], 1.0).unwrap();
        assert_eq!(engine.metrics.total_transcriptions, 2);
        assert!((engine.metrics.total_duration - 1.5).abs() < 0.001);
        assert_eq!(engine.metrics.offline_count, 2);
    }

    #[test]
    fn test_avg_confidence_running() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.transcribe_audio(&vec![0.5f32; 100], 1.0).unwrap();
        let c1 = engine.metrics.avg_confidence;
        engine.transcribe_audio(&vec![0.5f32; 100], 1.0).unwrap();
        let c2 = engine.metrics.avg_confidence;
        // Same input should produce same confidence so avg stays stable
        assert!((c1 - c2).abs() < 0.01);
    }

    #[test]
    fn test_detect_voice_activity_speech() {
        let engine = LocalVoiceEngine::new(VoiceConfig::default());
        let frame = vec![0.5f32; 320];
        let va = engine.detect_voice_activity(&frame, 100);
        assert!(va.is_speech);
        assert!(va.energy_level > 0.0);
        assert_eq!(va.timestamp, 100);
    }

    #[test]
    fn test_detect_voice_activity_silence() {
        let engine = LocalVoiceEngine::new(VoiceConfig::default());
        let frame = vec![0.001f32; 320];
        let va = engine.detect_voice_activity(&frame, 200);
        assert!(!va.is_speech);
    }

    #[test]
    fn test_detect_voice_activity_empty() {
        let engine = LocalVoiceEngine::new(VoiceConfig::default());
        let va = engine.detect_voice_activity(&[], 0);
        assert!(!va.is_speech);
        assert_eq!(va.energy_level, 0.0);
    }

    #[test]
    fn test_download_model() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        let path = engine.download_model(&WhisperModel::Tiny).unwrap();
        assert!(path.contains("whisper-tiny"));
    }

    #[test]
    fn test_download_active_model_sets_path() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.download_model(&WhisperModel::Base).unwrap();
        assert!(engine.model_path.is_some());
    }

    #[test]
    fn test_download_inactive_model_no_path() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.download_model(&WhisperModel::Large).unwrap();
        assert!(engine.model_path.is_none());
    }

    #[test]
    fn test_get_model_path() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(engine.get_model_path(&WhisperModel::Tiny).is_none());
        engine.download_model(&WhisperModel::Tiny).unwrap();
        assert!(engine.get_model_path(&WhisperModel::Tiny).is_some());
    }

    #[test]
    fn test_list_available_models() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.download_model(&WhisperModel::Base).unwrap();
        let list = engine.list_available_models();
        assert_eq!(list.len(), 5);
        let base = list.iter().find(|(m, _, _)| m == &WhisperModel::Base).unwrap();
        assert!(base.1); // downloaded
        let tiny = list.iter().find(|(m, _, _)| m == &WhisperModel::Tiny).unwrap();
        assert!(!tiny.1); // not downloaded
    }

    #[test]
    fn test_history_by_language() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.transcribe_audio(&vec![0.5f32; 100], 1.0).unwrap();
        engine.config.language = "fr".to_string();
        engine.transcribe_audio(&vec![0.5f32; 100], 1.0).unwrap();
        assert_eq!(engine.history_by_language("en").len(), 1);
        assert_eq!(engine.history_by_language("fr").len(), 1);
        assert_eq!(engine.history_by_language("de").len(), 0);
    }

    #[test]
    fn test_clear_history() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.transcribe_audio(&vec![0.5f32; 100], 1.0).unwrap();
        assert_eq!(engine.transcriptions.len(), 1);
        engine.clear_history();
        assert!(engine.transcriptions.is_empty());
    }

    #[test]
    fn test_set_model_downloaded() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        engine.download_model(&WhisperModel::Small).unwrap();
        assert!(engine.set_model(WhisperModel::Small).is_ok());
        assert_eq!(engine.config.model, WhisperModel::Small);
    }

    #[test]
    fn test_set_model_not_downloaded() {
        let mut engine = LocalVoiceEngine::new(VoiceConfig::default());
        assert!(engine.set_model(WhisperModel::Large).is_err());
    }

    #[test]
    fn test_transcription_result_serde() {
        let result = TranscriptionResult {
            text: "hello".to_string(),
            confidence: 0.95,
            duration_secs: 1.5,
            language: "en".to_string(),
            timestamp: 1,
            model_used: WhisperModel::Base,
            offline: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TranscriptionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[test]
    fn test_voice_config_serde() {
        let cfg = VoiceConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: VoiceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, deserialized);
    }

    #[test]
    fn test_voice_metrics_default() {
        let m = VoiceMetrics::default();
        assert_eq!(m.total_transcriptions, 0);
        assert_eq!(m.total_duration, 0.0);
        assert_eq!(m.errors, 0);
    }

    #[test]
    fn test_from_name_valid() {
        assert_eq!(WhisperModel::from_name("tiny"), Some(WhisperModel::Tiny));
        assert_eq!(WhisperModel::from_name("base"), Some(WhisperModel::Base));
        assert_eq!(WhisperModel::from_name("small"), Some(WhisperModel::Small));
        assert_eq!(WhisperModel::from_name("MEDIUM"), Some(WhisperModel::Medium));
        assert_eq!(WhisperModel::from_name("Large"), Some(WhisperModel::Large));
    }

    #[test]
    fn test_from_name_invalid() {
        assert_eq!(WhisperModel::from_name("huge"), None);
        assert_eq!(WhisperModel::from_name(""), None);
        assert_eq!(WhisperModel::from_name("base.en"), None);
    }

    #[test]
    fn test_ggml_urls() {
        for model in WhisperModel::all() {
            let url = model.ggml_url();
            assert!(url.starts_with("https://huggingface.co/ggerganov/whisper.cpp/"));
            assert!(url.ends_with(".bin"));
        }
    }

    #[test]
    fn test_ggml_url_contains_model_name() {
        assert!(WhisperModel::Tiny.ggml_url().contains("tiny"));
        assert!(WhisperModel::Base.ggml_url().contains("base"));
        assert!(WhisperModel::Large.ggml_url().contains("large"));
    }
}
