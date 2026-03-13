//! Multi-modal unified agent combining voice + vision + code.
//!
//! Provides a unified interface for processing text, voice, image, and code
//! inputs within a single agent context, with automatic mode detection
//! and action dispatch.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputMode {
    Text,
    Voice,
    Image,
    Code,
    Mixed,
}

impl InputMode {
    pub fn label(&self) -> &str {
        match self {
            InputMode::Text => "text",
            InputMode::Voice => "voice",
            InputMode::Image => "image",
            InputMode::Code => "code",
            InputMode::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiModalInput {
    pub mode: InputMode,
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub voice_transcript: Option<String>,
    pub code_snippet: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentAction {
    EditFile {
        path: String,
        content: String,
    },
    RunCommand {
        cmd: String,
    },
    GenerateCode {
        language: String,
        code: String,
    },
    DescribeImage {
        description: String,
    },
    TranscribeVoice {
        text: String,
    },
    SearchCode {
        query: String,
        results: Vec<String>,
    },
}

impl AgentAction {
    pub fn action_type(&self) -> &str {
        match self {
            AgentAction::EditFile { .. } => "edit_file",
            AgentAction::RunCommand { .. } => "run_command",
            AgentAction::GenerateCode { .. } => "generate_code",
            AgentAction::DescribeImage { .. } => "describe_image",
            AgentAction::TranscribeVoice { .. } => "transcribe_voice",
            AgentAction::SearchCode { .. } => "search_code",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnifiedTurn {
    pub id: String,
    pub inputs: Vec<MultiModalInput>,
    pub response: Option<String>,
    pub actions: Vec<AgentAction>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiModalContext {
    pub turns: Vec<UnifiedTurn>,
    pub active_files: Vec<String>,
    pub mode_history: Vec<InputMode>,
    pub total_images: usize,
    pub total_voice_segments: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiModalConfig {
    pub max_context_turns: usize,
    pub voice_enabled: bool,
    pub vision_enabled: bool,
    pub auto_detect_mode: bool,
    pub preferred_mode: InputMode,
}

impl Default for MultiModalConfig {
    fn default() -> Self {
        Self {
            max_context_turns: 100,
            voice_enabled: true,
            vision_enabled: true,
            auto_detect_mode: true,
            preferred_mode: InputMode::Text,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModeDetectionResult {
    pub detected_mode: InputMode,
    pub confidence: f64,
    pub indicators: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UnifiedAgent {
    pub context: MultiModalContext,
    pub config: MultiModalConfig,
    pub capabilities: Vec<InputMode>,
    next_turn_id: u64,
}

impl UnifiedAgent {
    pub fn new(config: MultiModalConfig) -> Self {
        let mut capabilities = vec![InputMode::Text, InputMode::Code, InputMode::Mixed];
        if config.voice_enabled {
            capabilities.push(InputMode::Voice);
        }
        if config.vision_enabled {
            capabilities.push(InputMode::Image);
        }

        Self {
            context: MultiModalContext {
                turns: Vec::new(),
                active_files: Vec::new(),
                mode_history: Vec::new(),
                total_images: 0,
                total_voice_segments: 0,
            },
            config,
            capabilities,
            next_turn_id: 1,
        }
    }

    pub fn add_input(&mut self, input: MultiModalInput) -> Result<String, String> {
        if !self.supports_mode(&input.mode) {
            return Err(format!("Mode {:?} is not supported", input.mode));
        }

        match &input.mode {
            InputMode::Image => self.context.total_images += 1,
            InputMode::Voice => self.context.total_voice_segments += 1,
            InputMode::Mixed => {
                if input.image_path.is_some() {
                    self.context.total_images += 1;
                }
                if input.voice_transcript.is_some() {
                    self.context.total_voice_segments += 1;
                }
            }
            _ => {}
        }

        self.context.mode_history.push(input.mode.clone());

        if let Some(ref path) = input.image_path {
            if !self.context.active_files.contains(path) {
                self.context.active_files.push(path.clone());
            }
        }

        let turn_id = format!("turn-{}", self.next_turn_id);
        self.next_turn_id += 1;

        let turn = UnifiedTurn {
            id: turn_id.clone(),
            timestamp: input.timestamp,
            inputs: vec![input],
            response: None,
            actions: Vec::new(),
        };

        self.context.turns.push(turn);

        if self.context.turns.len() > self.config.max_context_turns {
            let excess = self.context.turns.len() - self.config.max_context_turns;
            self.context.turns.drain(..excess);
        }

        Ok(turn_id)
    }

    pub fn detect_input_mode(text: &str) -> ModeDetectionResult {
        let mut indicators = Vec::new();
        let mut scores: HashMap<InputMode, f64> = HashMap::new();

        let image_extensions = [".png", ".jpg", ".jpeg", ".gif", ".bmp", ".svg", ".webp"];
        for ext in &image_extensions {
            if text.contains(ext) {
                *scores.entry(InputMode::Image).or_insert(0.0) += 0.4;
                indicators.push(format!("Contains image path (*{})", ext));
            }
        }
        if text.contains("screenshot") || text.contains("image") || text.contains("picture") {
            *scores.entry(InputMode::Image).or_insert(0.0) += 0.3;
            indicators.push("Contains image-related keyword".to_string());
        }

        let voice_keywords = [
            "hey ",
            "okay ",
            "listen",
            "dictate",
            "speak",
            "voice",
            "transcribe",
        ];
        for kw in &voice_keywords {
            if text.to_lowercase().contains(kw) {
                *scores.entry(InputMode::Voice).or_insert(0.0) += 0.3;
                indicators.push(format!("Contains voice keyword: {}", kw.trim()));
            }
        }

        let code_patterns = [
            "fn ",
            "def ",
            "class ",
            "function ",
            "pub ",
            "impl ",
            "struct ",
            "const ",
            "let ",
            "var ",
            "import ",
            "from ",
            "#include",
            "package ",
            "->",
            "=>",
            "(){",
            "};",
            "#!/",
        ];
        let mut code_hits = 0;
        for pat in &code_patterns {
            if text.contains(pat) {
                code_hits += 1;
            }
        }
        if code_hits > 0 {
            let code_score = (code_hits as f64 * 0.2).min(1.0);
            *scores.entry(InputMode::Code).or_insert(0.0) += code_score;
            indicators.push(format!("Contains {} code pattern(s)", code_hits));
        }

        let lines: Vec<&str> = text.lines().collect();
        if lines.len() > 3 {
            let indented = lines
                .iter()
                .filter(|l| l.starts_with("  ") || l.starts_with('\t'))
                .count();
            if indented > lines.len() / 2 {
                *scores.entry(InputMode::Code).or_insert(0.0) += 0.3;
                indicators.push("Multi-line indented text".to_string());
            }
        }

        *scores.entry(InputMode::Text).or_insert(0.0) += 0.1;

        let (detected_mode, confidence) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(mode, score)| (mode.clone(), *score))
            .unwrap_or((InputMode::Text, 0.1));

        let significant_modes = scores.iter().filter(|(_, s)| **s >= 0.3).count();
        if significant_modes > 1 {
            return ModeDetectionResult {
                detected_mode: InputMode::Mixed,
                confidence: confidence.min(1.0),
                indicators,
            };
        }

        ModeDetectionResult {
            detected_mode,
            confidence: confidence.min(1.0),
            indicators,
        }
    }

    pub fn process_turn(&mut self, turn_id: &str) -> Vec<AgentAction> {
        let turn = match self.context.turns.iter().find(|t| t.id == turn_id) {
            Some(t) => t.clone(),
            None => return Vec::new(),
        };

        let mut actions = Vec::new();

        for input in &turn.inputs {
            match &input.mode {
                InputMode::Image => {
                    if let Some(ref path) = input.image_path {
                        actions.push(AgentAction::DescribeImage {
                            description: format!("Analyzing image: {}", path),
                        });
                    }
                }
                InputMode::Voice => {
                    if let Some(ref transcript) = input.voice_transcript {
                        actions.push(AgentAction::TranscribeVoice {
                            text: transcript.clone(),
                        });
                    }
                }
                InputMode::Code => {
                    if let Some(ref snippet) = input.code_snippet {
                        let lang = if snippet.contains("fn ") || snippet.contains("pub ") {
                            "rust"
                        } else if snippet.contains("def ") {
                            "python"
                        } else if snippet.contains("function ") || snippet.contains("const ") {
                            "javascript"
                        } else {
                            "unknown"
                        };
                        actions.push(AgentAction::GenerateCode {
                            language: lang.to_string(),
                            code: snippet.clone(),
                        });
                    }
                }
                InputMode::Text => {
                    if let Some(ref text) = input.text {
                        if text.starts_with("search ") || text.starts_with("find ") {
                            let query = text.splitn(2, ' ').nth(1).unwrap_or("").to_string();
                            actions.push(AgentAction::SearchCode {
                                query,
                                results: Vec::new(),
                            });
                        } else if text.starts_with("run ") {
                            let cmd = text.splitn(2, ' ').nth(1).unwrap_or("").to_string();
                            actions.push(AgentAction::RunCommand { cmd });
                        }
                    }
                }
                InputMode::Mixed => {
                    if let Some(ref path) = input.image_path {
                        actions.push(AgentAction::DescribeImage {
                            description: format!("Analyzing image: {}", path),
                        });
                    }
                    if let Some(ref transcript) = input.voice_transcript {
                        actions.push(AgentAction::TranscribeVoice {
                            text: transcript.clone(),
                        });
                    }
                    if let Some(ref snippet) = input.code_snippet {
                        actions.push(AgentAction::GenerateCode {
                            language: "unknown".to_string(),
                            code: snippet.clone(),
                        });
                    }
                }
            }
        }

        if let Some(t) = self.context.turns.iter_mut().find(|t| t.id == turn_id) {
            t.actions = actions.clone();
        }

        actions
    }

    pub fn get_context(&self) -> &MultiModalContext {
        &self.context
    }

    pub fn get_turn(&self, id: &str) -> Option<&UnifiedTurn> {
        self.context.turns.iter().find(|t| t.id == id)
    }

    pub fn set_response(&mut self, turn_id: &str, response: String) -> Result<(), String> {
        let turn = self
            .context
            .turns
            .iter_mut()
            .find(|t| t.id == turn_id)
            .ok_or_else(|| format!("Turn not found: {}", turn_id))?;
        turn.response = Some(response);
        Ok(())
    }

    pub fn add_action(&mut self, turn_id: &str, action: AgentAction) -> Result<(), String> {
        let turn = self
            .context
            .turns
            .iter_mut()
            .find(|t| t.id == turn_id)
            .ok_or_else(|| format!("Turn not found: {}", turn_id))?;
        turn.actions.push(action);
        Ok(())
    }

    pub fn mode_summary(&self) -> HashMap<InputMode, usize> {
        let mut summary = HashMap::new();
        for mode in &self.context.mode_history {
            *summary.entry(mode.clone()).or_insert(0) += 1;
        }
        summary
    }

    pub fn supports_mode(&self, mode: &InputMode) -> bool {
        self.capabilities.contains(mode)
    }

    pub fn clear_context(&mut self) {
        self.context.turns.clear();
        self.context.active_files.clear();
        self.context.mode_history.clear();
        self.context.total_images = 0;
        self.context.total_voice_segments = 0;
        self.next_turn_id = 1;
    }

    pub fn context_token_estimate(&self) -> usize {
        let mut total = 0usize;
        for turn in &self.context.turns {
            for input in &turn.inputs {
                if let Some(ref t) = input.text {
                    total += t.len() / 4;
                }
                if let Some(ref t) = input.voice_transcript {
                    total += t.len() / 4;
                }
                if let Some(ref c) = input.code_snippet {
                    total += c.len() / 4;
                }
                if input.image_path.is_some() {
                    total += 1000;
                }
            }
            if let Some(ref r) = turn.response {
                total += r.len() / 4;
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_agent() -> UnifiedAgent {
        UnifiedAgent::new(MultiModalConfig::default())
    }

    fn text_input(text: &str) -> MultiModalInput {
        MultiModalInput {
            mode: InputMode::Text,
            text: Some(text.to_string()),
            image_path: None,
            voice_transcript: None,
            code_snippet: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn test_new_agent() {
        let agent = default_agent();
        assert!(agent.context.turns.is_empty());
        assert_eq!(agent.capabilities.len(), 5);
    }

    #[test]
    fn test_new_agent_no_voice() {
        let config = MultiModalConfig {
            voice_enabled: false,
            ..Default::default()
        };
        let agent = UnifiedAgent::new(config);
        assert!(!agent.supports_mode(&InputMode::Voice));
    }

    #[test]
    fn test_new_agent_no_vision() {
        let config = MultiModalConfig {
            vision_enabled: false,
            ..Default::default()
        };
        let agent = UnifiedAgent::new(config);
        assert!(!agent.supports_mode(&InputMode::Image));
    }

    #[test]
    fn test_add_text_input() {
        let mut agent = default_agent();
        let id = agent.add_input(text_input("hello")).unwrap();
        assert_eq!(id, "turn-1");
        assert_eq!(agent.context.turns.len(), 1);
    }

    #[test]
    fn test_add_image_input_tracks_count() {
        let mut agent = default_agent();
        agent
            .add_input(MultiModalInput {
                mode: InputMode::Image,
                text: None,
                image_path: Some("/tmp/screenshot.png".to_string()),
                voice_transcript: None,
                code_snippet: None,
                timestamp: 1000,
            })
            .unwrap();
        assert_eq!(agent.context.total_images, 1);
        assert!(agent
            .context
            .active_files
            .contains(&"/tmp/screenshot.png".to_string()));
    }

    #[test]
    fn test_add_voice_input_tracks_count() {
        let mut agent = default_agent();
        agent
            .add_input(MultiModalInput {
                mode: InputMode::Voice,
                text: None,
                image_path: None,
                voice_transcript: Some("Hello world".to_string()),
                code_snippet: None,
                timestamp: 1000,
            })
            .unwrap();
        assert_eq!(agent.context.total_voice_segments, 1);
    }

    #[test]
    fn test_add_input_unsupported_mode() {
        let config = MultiModalConfig {
            voice_enabled: false,
            ..Default::default()
        };
        let mut agent = UnifiedAgent::new(config);
        let result = agent.add_input(MultiModalInput {
            mode: InputMode::Voice,
            text: None,
            image_path: None,
            voice_transcript: Some("test".to_string()),
            code_snippet: None,
            timestamp: 1000,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_add_input_enforces_max_turns() {
        let config = MultiModalConfig {
            max_context_turns: 3,
            ..Default::default()
        };
        let mut agent = UnifiedAgent::new(config);
        for i in 0..5 {
            agent
                .add_input(text_input(&format!("msg {}", i)))
                .unwrap();
        }
        assert_eq!(agent.context.turns.len(), 3);
    }

    #[test]
    fn test_unique_turn_ids() {
        let mut agent = default_agent();
        let id1 = agent.add_input(text_input("one")).unwrap();
        let id2 = agent.add_input(text_input("two")).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_detect_mode_text() {
        let result = UnifiedAgent::detect_input_mode("Hello, how are you?");
        assert_eq!(result.detected_mode, InputMode::Text);
    }

    #[test]
    fn test_detect_mode_image() {
        let result = UnifiedAgent::detect_input_mode("Check this file /tmp/photo.png");
        assert_eq!(result.detected_mode, InputMode::Image);
        assert!(!result.indicators.is_empty());
    }

    #[test]
    fn test_detect_mode_voice() {
        let result = UnifiedAgent::detect_input_mode("hey transcribe this voice message");
        assert!(result.indicators.iter().any(|i| i.contains("voice")));
    }

    #[test]
    fn test_detect_mode_code() {
        let code = "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}";
        let result = UnifiedAgent::detect_input_mode(code);
        assert!(
            result.detected_mode == InputMode::Code
                || result.detected_mode == InputMode::Mixed
        );
    }

    #[test]
    fn test_detect_mode_confidence() {
        let result = UnifiedAgent::detect_input_mode("simple text");
        assert!(result.confidence > 0.0);
        assert!(result.confidence <= 1.0);
    }

    #[test]
    fn test_process_turn_text_search() {
        let mut agent = default_agent();
        let id = agent
            .add_input(text_input("search for_bar_function"))
            .unwrap();
        let actions = agent.process_turn(&id);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type(), "search_code");
    }

    #[test]
    fn test_process_turn_text_run() {
        let mut agent = default_agent();
        let id = agent.add_input(text_input("run cargo test")).unwrap();
        let actions = agent.process_turn(&id);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            AgentAction::RunCommand { cmd } => assert_eq!(cmd, "cargo test"),
            _ => panic!("Expected RunCommand"),
        }
    }

    #[test]
    fn test_process_turn_image() {
        let mut agent = default_agent();
        let id = agent
            .add_input(MultiModalInput {
                mode: InputMode::Image,
                text: None,
                image_path: Some("/tmp/ui.png".to_string()),
                voice_transcript: None,
                code_snippet: None,
                timestamp: 1000,
            })
            .unwrap();
        let actions = agent.process_turn(&id);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type(), "describe_image");
    }

    #[test]
    fn test_process_turn_voice() {
        let mut agent = default_agent();
        let id = agent
            .add_input(MultiModalInput {
                mode: InputMode::Voice,
                text: None,
                image_path: None,
                voice_transcript: Some("Create a new function".to_string()),
                code_snippet: None,
                timestamp: 1000,
            })
            .unwrap();
        let actions = agent.process_turn(&id);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type(), "transcribe_voice");
    }

    #[test]
    fn test_process_turn_code_rust() {
        let mut agent = default_agent();
        let id = agent
            .add_input(MultiModalInput {
                mode: InputMode::Code,
                text: None,
                image_path: None,
                voice_transcript: None,
                code_snippet: Some("fn hello() {}".to_string()),
                timestamp: 1000,
            })
            .unwrap();
        let actions = agent.process_turn(&id);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            AgentAction::GenerateCode { language, .. } => assert_eq!(language, "rust"),
            _ => panic!("Expected GenerateCode"),
        }
    }

    #[test]
    fn test_process_turn_code_python() {
        let mut agent = default_agent();
        let id = agent
            .add_input(MultiModalInput {
                mode: InputMode::Code,
                text: None,
                image_path: None,
                voice_transcript: None,
                code_snippet: Some("def hello():\n    pass".to_string()),
                timestamp: 1000,
            })
            .unwrap();
        let actions = agent.process_turn(&id);
        match &actions[0] {
            AgentAction::GenerateCode { language, .. } => assert_eq!(language, "python"),
            _ => panic!("Expected GenerateCode"),
        }
    }

    #[test]
    fn test_process_turn_mixed() {
        let mut agent = default_agent();
        let id = agent
            .add_input(MultiModalInput {
                mode: InputMode::Mixed,
                text: Some("Analyze this".to_string()),
                image_path: Some("/tmp/screen.png".to_string()),
                voice_transcript: Some("Fix the bug".to_string()),
                code_snippet: Some("let x = 1;".to_string()),
                timestamp: 1000,
            })
            .unwrap();
        let actions = agent.process_turn(&id);
        assert_eq!(actions.len(), 3); // image + voice + code
    }

    #[test]
    fn test_process_turn_not_found() {
        let mut agent = default_agent();
        let actions = agent.process_turn("nonexistent");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_get_turn() {
        let mut agent = default_agent();
        let id = agent.add_input(text_input("hello")).unwrap();
        assert!(agent.get_turn(&id).is_some());
        assert!(agent.get_turn("nonexistent").is_none());
    }

    #[test]
    fn test_set_response() {
        let mut agent = default_agent();
        let id = agent.add_input(text_input("hello")).unwrap();
        agent.set_response(&id, "world".to_string()).unwrap();
        let turn = agent.get_turn(&id).unwrap();
        assert_eq!(turn.response.as_deref(), Some("world"));
    }

    #[test]
    fn test_set_response_not_found() {
        let mut agent = default_agent();
        assert!(agent.set_response("bad", "x".to_string()).is_err());
    }

    #[test]
    fn test_add_action() {
        let mut agent = default_agent();
        let id = agent.add_input(text_input("hello")).unwrap();
        agent
            .add_action(
                &id,
                AgentAction::RunCommand {
                    cmd: "ls".to_string(),
                },
            )
            .unwrap();
        let turn = agent.get_turn(&id).unwrap();
        assert_eq!(turn.actions.len(), 1);
    }

    #[test]
    fn test_add_action_not_found() {
        let mut agent = default_agent();
        assert!(agent
            .add_action(
                "bad",
                AgentAction::RunCommand {
                    cmd: "ls".to_string(),
                },
            )
            .is_err());
    }

    #[test]
    fn test_mode_summary() {
        let mut agent = default_agent();
        agent.add_input(text_input("a")).unwrap();
        agent.add_input(text_input("b")).unwrap();
        agent
            .add_input(MultiModalInput {
                mode: InputMode::Code,
                text: None,
                image_path: None,
                voice_transcript: None,
                code_snippet: Some("x".to_string()),
                timestamp: 1000,
            })
            .unwrap();
        let summary = agent.mode_summary();
        assert_eq!(summary.get(&InputMode::Text), Some(&2));
        assert_eq!(summary.get(&InputMode::Code), Some(&1));
    }

    #[test]
    fn test_supports_mode() {
        let agent = default_agent();
        assert!(agent.supports_mode(&InputMode::Text));
        assert!(agent.supports_mode(&InputMode::Voice));
        assert!(agent.supports_mode(&InputMode::Image));
        assert!(agent.supports_mode(&InputMode::Code));
        assert!(agent.supports_mode(&InputMode::Mixed));
    }

    #[test]
    fn test_clear_context() {
        let mut agent = default_agent();
        agent.add_input(text_input("hello")).unwrap();
        agent
            .add_input(MultiModalInput {
                mode: InputMode::Image,
                text: None,
                image_path: Some("/tmp/x.png".to_string()),
                voice_transcript: None,
                code_snippet: None,
                timestamp: 1,
            })
            .unwrap();
        agent.clear_context();
        assert!(agent.context.turns.is_empty());
        assert!(agent.context.active_files.is_empty());
        assert!(agent.context.mode_history.is_empty());
        assert_eq!(agent.context.total_images, 0);
        assert_eq!(agent.context.total_voice_segments, 0);
    }

    #[test]
    fn test_context_token_estimate_text() {
        let mut agent = default_agent();
        agent
            .add_input(text_input("This is a test message with forty chars!"))
            .unwrap();
        let est = agent.context_token_estimate();
        assert!(est > 0);
        assert_eq!(est, 10); // 40 / 4
    }

    #[test]
    fn test_context_token_estimate_image() {
        let mut agent = default_agent();
        agent
            .add_input(MultiModalInput {
                mode: InputMode::Image,
                text: None,
                image_path: Some("/tmp/x.png".to_string()),
                voice_transcript: None,
                code_snippet: None,
                timestamp: 1,
            })
            .unwrap();
        let est = agent.context_token_estimate();
        assert_eq!(est, 1000);
    }

    #[test]
    fn test_context_token_estimate_with_response() {
        let mut agent = default_agent();
        let id = agent.add_input(text_input("hi")).unwrap();
        agent
            .set_response(&id, "This is a response message.".to_string())
            .unwrap();
        let est = agent.context_token_estimate();
        // "hi" = 0 (2/4=0) + "This is a response message." = 27/4 = 6
        assert!(est >= 6);
    }

    #[test]
    fn test_mode_history_tracking() {
        let mut agent = default_agent();
        agent.add_input(text_input("a")).unwrap();
        agent
            .add_input(MultiModalInput {
                mode: InputMode::Code,
                text: None,
                image_path: None,
                voice_transcript: None,
                code_snippet: Some("x".to_string()),
                timestamp: 1,
            })
            .unwrap();
        agent.add_input(text_input("b")).unwrap();
        assert_eq!(agent.context.mode_history.len(), 3);
        assert_eq!(agent.context.mode_history[0], InputMode::Text);
        assert_eq!(agent.context.mode_history[1], InputMode::Code);
        assert_eq!(agent.context.mode_history[2], InputMode::Text);
    }

    #[test]
    fn test_default_config() {
        let cfg = MultiModalConfig::default();
        assert_eq!(cfg.max_context_turns, 100);
        assert!(cfg.voice_enabled);
        assert!(cfg.vision_enabled);
        assert!(cfg.auto_detect_mode);
        assert_eq!(cfg.preferred_mode, InputMode::Text);
    }

    #[test]
    fn test_input_mode_labels() {
        assert_eq!(InputMode::Text.label(), "text");
        assert_eq!(InputMode::Voice.label(), "voice");
        assert_eq!(InputMode::Image.label(), "image");
        assert_eq!(InputMode::Code.label(), "code");
        assert_eq!(InputMode::Mixed.label(), "mixed");
    }

    #[test]
    fn test_agent_action_types() {
        assert_eq!(
            AgentAction::EditFile {
                path: String::new(),
                content: String::new(),
            }
            .action_type(),
            "edit_file"
        );
        assert_eq!(
            AgentAction::RunCommand {
                cmd: String::new(),
            }
            .action_type(),
            "run_command"
        );
        assert_eq!(
            AgentAction::GenerateCode {
                language: String::new(),
                code: String::new(),
            }
            .action_type(),
            "generate_code"
        );
    }

    #[test]
    fn test_mixed_input_increments_both_counters() {
        let mut agent = default_agent();
        agent
            .add_input(MultiModalInput {
                mode: InputMode::Mixed,
                text: Some("describe and transcribe".to_string()),
                image_path: Some("/tmp/x.png".to_string()),
                voice_transcript: Some("hello".to_string()),
                code_snippet: None,
                timestamp: 1,
            })
            .unwrap();
        assert_eq!(agent.context.total_images, 1);
        assert_eq!(agent.context.total_voice_segments, 1);
    }

    #[test]
    fn test_no_duplicate_active_files() {
        let mut agent = default_agent();
        for _ in 0..3 {
            agent
                .add_input(MultiModalInput {
                    mode: InputMode::Image,
                    text: None,
                    image_path: Some("/tmp/same.png".to_string()),
                    voice_transcript: None,
                    code_snippet: None,
                    timestamp: 1,
                })
                .unwrap();
        }
        assert_eq!(agent.context.active_files.len(), 1);
    }
}
