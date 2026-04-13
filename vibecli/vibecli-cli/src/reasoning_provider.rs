//! reasoning_provider — Extended reasoning / thinking-block support for Claude.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThinkingBlock {
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelTier { #[default] Standard, Reasoning, Extended }

#[derive(Debug, Clone, Default)]
pub struct ReasoningBudget { pub max_thinking_tokens: u32 }

#[derive(Debug, Clone, Default)]
pub struct ReasoningConfig { pub tier: ModelTier, pub strip_thinking: bool }

impl ReasoningConfig {
    pub fn new(tier: ModelTier) -> Self { Self { tier, strip_thinking: false } }
    pub fn with_strip_thinking(mut self, strip: bool) -> Self { self.strip_thinking = strip; self }
}

#[derive(Debug, Clone, Default)]
pub struct ReasoningResponse {
    pub response: String,
    pub thinking_blocks: Vec<ThinkingBlock>,
}

pub fn parse_thinking_blocks(raw: &str) -> Vec<ThinkingBlock> {
    let mut blocks = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find("<thinking>") {
        rest = &rest[start + "<thinking>".len()..];
        if let Some(end) = rest.find("</thinking>") {
            blocks.push(ThinkingBlock { content: rest[..end].to_string() });
            rest = &rest[end + "</thinking>".len()..];
        } else { break; }
    }
    blocks
}

pub fn strip_thinking_from(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut rest = raw;
    loop {
        match rest.find("<thinking>") {
            None => { result.push_str(rest); break; }
            Some(start) => {
                result.push_str(&rest[..start]);
                rest = &rest[start + "<thinking>".len()..];
                if let Some(end) = rest.find("</thinking>") {
                    rest = &rest[end + "</thinking>".len()..];
                } else { break; }
            }
        }
    }
    result.trim().to_string()
}

pub fn token_budget_for_complexity(complexity: u8) -> ReasoningBudget {
    let max_thinking_tokens = match complexity {
        0..=2 => 1_024, 3..=4 => 4_096, 5..=6 => 8_192, _ => 16_384,
    };
    ReasoningBudget { max_thinking_tokens }
}

pub fn build_reasoning_response(raw: &str, config: &ReasoningConfig) -> ReasoningResponse {
    let thinking_blocks = parse_thinking_blocks(raw);
    let response = if config.strip_thinking { strip_thinking_from(raw) } else { raw.to_string() };
    ReasoningResponse { response, thinking_blocks }
}
