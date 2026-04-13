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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_thinking_blocks_empty() {
        assert!(parse_thinking_blocks("no thinking here").is_empty());
    }

    #[test]
    fn test_parse_thinking_blocks_one() {
        let raw = "before<thinking>inner thought</thinking>after";
        let blocks = parse_thinking_blocks(raw);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "inner thought");
    }

    #[test]
    fn test_parse_thinking_blocks_two() {
        let raw = "<thinking>A</thinking>mid<thinking>B</thinking>";
        let blocks = parse_thinking_blocks(raw);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "A");
        assert_eq!(blocks[1].content, "B");
    }

    #[test]
    fn test_strip_thinking_basic() {
        let raw = "Result: <thinking>hidden</thinking>42";
        assert_eq!(strip_thinking_from(raw), "Result: 42");
    }

    #[test]
    fn test_strip_thinking_no_block() {
        assert_eq!(strip_thinking_from("plain text"), "plain text");
    }

    #[test]
    fn test_token_budget_low_complexity() {
        assert_eq!(token_budget_for_complexity(1).max_thinking_tokens, 1024);
    }

    #[test]
    fn test_token_budget_high_complexity() {
        assert!(token_budget_for_complexity(9).max_thinking_tokens >= 16_384);
    }

    #[test]
    fn test_build_response_strips_thinking() {
        let raw = "<thinking>plan</thinking>answer";
        let config = ReasoningConfig::new(ModelTier::Reasoning).with_strip_thinking(true);
        let resp = build_reasoning_response(raw, &config);
        assert_eq!(resp.response, "answer");
        assert_eq!(resp.thinking_blocks.len(), 1);
    }

    #[test]
    fn test_build_response_keeps_thinking() {
        let raw = "<thinking>plan</thinking>answer";
        let config = ReasoningConfig::new(ModelTier::Extended);
        let resp = build_reasoning_response(raw, &config);
        assert!(resp.response.contains("<thinking>"));
    }

    #[test]
    fn test_model_tier_default() {
        assert_eq!(ModelTier::default(), ModelTier::Standard);
    }
}
