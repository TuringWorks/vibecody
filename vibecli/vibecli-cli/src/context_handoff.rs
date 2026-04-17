//! Cross-provider context handoff — serializable session context.
//! Pi-mono gap bridge: Phase B1.
//!
//! Enables transferring a live conversation (system prompt + messages + tool
//! definitions) verbatim to a different AI provider mid-session. Typical uses:
//!   - Cost routing:     start on Sonnet, continue on Haiku once context is warm
//!   - Fallback:         primary provider down → switch transparently
//!   - Capability gap:   route tool-calling turns to Gemini, reasoning to Claude

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Role of a participant in a conversation turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Granular content sub-type inside a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentKind {
    Text,
    ToolCall,
    ToolResult,
    Image,
}

/// One logical piece of content within a message.
#[derive(Debug, Clone)]
pub struct ContentPart {
    pub kind: ContentKind,
    /// Plain text payload (Text parts, or stringified image description).
    pub text: Option<String>,
    /// Correlates a ToolCall with its ToolResult.
    pub tool_call_id: Option<String>,
    /// Name of the tool being called (ToolCall parts).
    pub tool_name: Option<String>,
    /// JSON-encoded tool arguments (ToolCall parts).
    pub tool_args: Option<String>,
    /// Text payload returned by a tool (ToolResult parts).
    pub tool_result: Option<String>,
}

impl ContentPart {
    /// Create a plain-text content part.
    pub fn text(t: impl Into<String>) -> Self {
        Self {
            kind: ContentKind::Text,
            text: Some(t.into()),
            tool_call_id: None,
            tool_name: None,
            tool_args: None,
            tool_result: None,
        }
    }

    /// Create a tool-call content part.
    pub fn tool_call(
        call_id: impl Into<String>,
        name: impl Into<String>,
        args: impl Into<String>,
    ) -> Self {
        Self {
            kind: ContentKind::ToolCall,
            text: None,
            tool_call_id: Some(call_id.into()),
            tool_name: Some(name.into()),
            tool_args: Some(args.into()),
            tool_result: None,
        }
    }

    /// Create a tool-result content part.
    pub fn tool_result(call_id: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            kind: ContentKind::ToolResult,
            text: None,
            tool_call_id: Some(call_id.into()),
            tool_name: None,
            tool_args: None,
            tool_result: Some(result.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// HandoffMessage
// ---------------------------------------------------------------------------

/// A single conversation turn — role + ordered content parts + free-form metadata.
#[derive(Debug, Clone)]
pub struct HandoffMessage {
    pub role: MessageRole,
    pub parts: Vec<ContentPart>,
    /// Arbitrary key/value pairs (e.g. model name, latency_ms, cost_usd).
    pub metadata: HashMap<String, String>,
}

impl HandoffMessage {
    /// Construct a User turn with a single text part.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            parts: vec![ContentPart::text(text)],
            metadata: HashMap::new(),
        }
    }

    /// Construct an Assistant turn with a single text part.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            parts: vec![ContentPart::text(text)],
            metadata: HashMap::new(),
        }
    }

    /// Construct a System turn with a single text part.
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            parts: vec![ContentPart::text(text)],
            metadata: HashMap::new(),
        }
    }

    /// Construct a Tool result turn.
    pub fn tool_result(call_id: &str, result: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            parts: vec![ContentPart::tool_result(call_id, result)],
            metadata: HashMap::new(),
        }
    }

    /// Concatenate all Text parts into a single string, separated by newlines.
    pub fn text_content(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| {
                if p.kind == ContentKind::Text {
                    p.text.as_deref()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns `true` if any part is a ToolCall.
    pub fn has_tool_calls(&self) -> bool {
        self.parts.iter().any(|p| p.kind == ContentKind::ToolCall)
    }

    /// Rough character count across all content for token estimation.
    fn char_len(&self) -> usize {
        self.parts.iter().fold(0usize, |acc, p| {
            acc + p.text.as_deref().map_or(0, str::len)
                + p.tool_args.as_deref().map_or(0, str::len)
                + p.tool_result.as_deref().map_or(0, str::len)
        })
    }
}

// ---------------------------------------------------------------------------
// ToolDefinition
// ---------------------------------------------------------------------------

/// Portable description of a single callable tool/function.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema string describing the function parameters.
    pub parameters_json: String,
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters_json: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters_json: parameters_json.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// HandoffContext
// ---------------------------------------------------------------------------

/// Portable context bundle that can be serialized and handed to any provider.
///
/// All fields are chosen to map unambiguously onto the request shape of every
/// major provider (OpenAI, Anthropic, Gemini, Groq, …).
#[derive(Debug, Clone)]
pub struct HandoffContext {
    /// Provider that originally produced this context (e.g. `"claude"`, `"openai"`).
    pub source_provider: String,
    /// Provider the context is destined for, when known.
    pub target_provider: Option<String>,
    /// Optional top-level system prompt (separate from messages so it can be
    /// injected as a dedicated field on providers that support it).
    pub system_prompt: Option<String>,
    pub messages: Vec<HandoffMessage>,
    pub tools: Vec<ToolDefinition>,
    pub metadata: HashMap<String, String>,
}

impl HandoffContext {
    /// Create an empty context attributed to `source`.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source_provider: source.into(),
            target_provider: None,
            system_prompt: None,
            messages: Vec::new(),
            tools: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Builder method — set the system prompt.
    pub fn with_system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Append a message to the conversation history.
    pub fn push_message(&mut self, msg: HandoffMessage) {
        self.messages.push(msg);
    }

    /// Register a tool definition.
    pub fn push_tool(&mut self, tool: ToolDefinition) {
        self.tools.push(tool);
    }

    /// Number of messages in the history.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Rough token estimate: total characters / 4 (industry heuristic).
    pub fn token_estimate(&self) -> usize {
        let sys_chars = self
            .system_prompt
            .as_deref()
            .map_or(0, str::len);
        let msg_chars: usize = self.messages.iter().map(HandoffMessage::char_len).sum();
        let tool_chars: usize = self
            .tools
            .iter()
            .map(|t| t.description.len() + t.parameters_json.len())
            .sum();
        (sys_chars + msg_chars + tool_chars) / 4
    }

    /// Serialize to a JSON string. Uses a hand-rolled encoder to avoid pulling
    /// in `serde_json` — the format is deliberately simple and round-trips via
    /// `deserialize`.
    pub fn serialize(&self) -> Result<String, String> {
        let mut out = String::with_capacity(512);
        out.push('{');

        // source_provider
        push_kv_str(&mut out, "source_provider", &self.source_provider);
        out.push(',');

        // target_provider
        match &self.target_provider {
            Some(t) => {
                push_key(&mut out, "target_provider");
                out.push('"');
                push_escaped(&mut out, t);
                out.push('"');
            }
            None => {
                push_key(&mut out, "target_provider");
                out.push_str("null");
            }
        }
        out.push(',');

        // system_prompt
        match &self.system_prompt {
            Some(s) => {
                push_key(&mut out, "system_prompt");
                out.push('"');
                push_escaped(&mut out, s);
                out.push('"');
            }
            None => {
                push_key(&mut out, "system_prompt");
                out.push_str("null");
            }
        }
        out.push(',');

        // messages
        push_key(&mut out, "messages");
        out.push('[');
        for (i, msg) in self.messages.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            serialize_message(&mut out, msg);
        }
        out.push(']');
        out.push(',');

        // tools
        push_key(&mut out, "tools");
        out.push('[');
        for (i, tool) in self.tools.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            push_kv_str(&mut out, "name", &tool.name);
            out.push(',');
            push_kv_str(&mut out, "description", &tool.description);
            out.push(',');
            push_kv_str(&mut out, "parameters_json", &tool.parameters_json);
            out.push('}');
        }
        out.push(']');
        out.push(',');

        // metadata
        push_key(&mut out, "metadata");
        serialize_map(&mut out, &self.metadata);

        out.push('}');
        Ok(out)
    }

    /// Deserialize from a JSON string produced by `serialize`.
    pub fn deserialize(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if !s.starts_with('{') || !s.ends_with('}') {
            return Err("expected JSON object".into());
        }
        // Use a simple key-extraction approach that works with our known schema.
        let source_provider = extract_str_field(s, "source_provider")
            .ok_or("missing source_provider")?;
        let target_provider = extract_optional_str_field(s, "target_provider");
        let system_prompt = extract_optional_str_field(s, "system_prompt");
        let messages = extract_messages(s)?;
        let tools = extract_tools(s)?;
        let metadata = extract_map_field(s, "metadata");

        Ok(Self {
            source_provider,
            target_provider,
            system_prompt,
            messages,
            tools,
            metadata,
        })
    }

    /// Clone this context and set `target_provider` to the given name.
    pub fn for_provider(&self, target: &str) -> Self {
        let mut cloned = self.clone();
        cloned.target_provider = Some(target.to_owned());
        cloned
    }

    /// Return a new context with the oldest messages dropped so that the
    /// remaining token estimate does not exceed `max_tokens`.
    /// The system prompt and tool definitions are always preserved.
    /// Messages are removed from the front (oldest first).
    pub fn trim_to_token_budget(&self, max_tokens: usize) -> Self {
        let mut trimmed = self.clone();
        while trimmed.token_estimate() > max_tokens && !trimmed.messages.is_empty() {
            trimmed.messages.remove(0);
        }
        trimmed
    }

    /// Return the last User-role message, if any.
    pub fn last_user_message(&self) -> Option<&HandoffMessage> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
    }

    /// Human-readable one-line summary, e.g. "12 messages, 3 tools, ~4200 tokens".
    pub fn summary(&self) -> String {
        format!(
            "{} messages, {} tools, ~{} tokens",
            self.messages.len(),
            self.tools.len(),
            self.token_estimate()
        )
    }
}

// ---------------------------------------------------------------------------
// HandoffHistory
// ---------------------------------------------------------------------------

/// Reason a context handoff was triggered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffReason {
    /// Cheaper provider selected to reduce per-token spend.
    CostRouting,
    /// Primary provider returned an error or was unreachable.
    Fallback,
    /// Current provider lacks a required capability (e.g. tool calling).
    CapabilityGap,
    /// The user explicitly requested a provider switch.
    UserRequested,
}

/// One recorded provider-switch event.
#[derive(Debug, Clone)]
pub struct HandoffEvent {
    pub from_provider: String,
    pub to_provider: String,
    pub reason: HandoffReason,
    pub message_count_at_handoff: usize,
    /// Wall-clock milliseconds since the Unix epoch (best-effort).
    pub timestamp_ms: u64,
}

/// Append-only log of every provider switch that occurred during a session.
#[derive(Debug, Default)]
pub struct HandoffHistory {
    entries: Vec<HandoffEvent>,
}

impl HandoffHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new handoff event. `msg_count` is the number of messages in
    /// the context at the moment the switch is triggered.
    pub fn record(
        &mut self,
        from: &str,
        to: &str,
        reason: HandoffReason,
        msg_count: usize,
    ) {
        self.entries.push(HandoffEvent {
            from_provider: from.to_owned(),
            to_provider: to.to_owned(),
            reason,
            message_count_at_handoff: msg_count,
            // Use a monotonic counter when std::time is unavailable in tests.
            timestamp_ms: timestamp_now_ms(),
        });
    }

    /// Total number of recorded handoff events.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Most recent handoff event, or `None` if no handoffs have been recorded.
    pub fn last(&self) -> Option<&HandoffEvent> {
        self.entries.last()
    }

    /// Unique providers seen across all events, in order of first appearance.
    /// Includes both `from_provider` and `to_provider` values.
    pub fn providers_used(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for ev in &self.entries {
            if !seen.contains(&ev.from_provider) {
                seen.push(ev.from_provider.clone());
            }
            if !seen.contains(&ev.to_provider) {
                seen.push(ev.to_provider.clone());
            }
        }
        seen
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers (no external crates)
// ---------------------------------------------------------------------------

fn push_key(out: &mut String, key: &str) {
    out.push('"');
    push_escaped(out, key);
    out.push_str("\":");
}

fn push_kv_str(out: &mut String, key: &str, val: &str) {
    push_key(out, key);
    out.push('"');
    push_escaped(out, val);
    out.push('"');
}

fn push_escaped(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
}

fn role_to_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

fn role_from_str(s: &str) -> MessageRole {
    match s {
        "system" => MessageRole::System,
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        _ => MessageRole::User,
    }
}

fn kind_to_str(kind: &ContentKind) -> &'static str {
    match kind {
        ContentKind::Text => "text",
        ContentKind::ToolCall => "tool_call",
        ContentKind::ToolResult => "tool_result",
        ContentKind::Image => "image",
    }
}

fn kind_from_str(s: &str) -> ContentKind {
    match s {
        "tool_call" => ContentKind::ToolCall,
        "tool_result" => ContentKind::ToolResult,
        "image" => ContentKind::Image,
        _ => ContentKind::Text,
    }
}

fn serialize_optional_str(out: &mut String, key: &str, val: &Option<String>) {
    push_key(out, key);
    match val {
        Some(v) => {
            out.push('"');
            push_escaped(out, v);
            out.push('"');
        }
        None => out.push_str("null"),
    }
}

fn serialize_part(out: &mut String, part: &ContentPart) {
    out.push('{');
    push_kv_str(out, "kind", kind_to_str(&part.kind));
    out.push(',');
    serialize_optional_str(out, "text", &part.text);
    out.push(',');
    serialize_optional_str(out, "tool_call_id", &part.tool_call_id);
    out.push(',');
    serialize_optional_str(out, "tool_name", &part.tool_name);
    out.push(',');
    serialize_optional_str(out, "tool_args", &part.tool_args);
    out.push(',');
    serialize_optional_str(out, "tool_result", &part.tool_result);
    out.push('}');
}

fn serialize_message(out: &mut String, msg: &HandoffMessage) {
    out.push('{');
    push_kv_str(out, "role", role_to_str(&msg.role));
    out.push(',');
    push_key(out, "parts");
    out.push('[');
    for (i, part) in msg.parts.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        serialize_part(out, part);
    }
    out.push(']');
    out.push(',');
    push_key(out, "metadata");
    serialize_map(out, &msg.metadata);
    out.push('}');
}

fn serialize_map(out: &mut String, map: &HashMap<String, String>) {
    out.push('{');
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_kv_str(out, k, v);
    }
    out.push('}');
}

// ---------------------------------------------------------------------------
// Deserialization helpers
// ---------------------------------------------------------------------------

/// Extract the value of a `"key":"value"` pair, returning the unescaped string.
fn extract_str_field(s: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start = s.find(needle.as_str())?;
    let after_colon = start + needle.len();
    let rest = s[after_colon..].trim_start();
    if rest.starts_with("null") {
        return None;
    }
    parse_json_string(rest)
}

fn extract_optional_str_field(s: &str, key: &str) -> Option<String> {
    extract_str_field(s, key)
}

/// Parse a JSON string literal at the start of `s`, returning the unescaped value.
fn parse_json_string(s: &str) -> Option<String> {
    let s = s.trim_start();
    if !s.starts_with('"') {
        return None;
    }
    let s = &s[1..];
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    loop {
        match chars.next()? {
            '"' => break,
            '\\' => match chars.next()? {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                c => result.push(c),
            },
            c => result.push(c),
        }
    }
    Some(result)
}

/// Locate the outermost `[...]` block for `key` and return the raw slice.
fn extract_array_raw<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\":[", key);
    // Also allow whitespace after colon
    let needle2 = format!("\"{}\":  [", key);
    let start = s.find(needle.as_str()).or_else(|| s.find(needle2.as_str()))?;
    let bracket_pos = start + key.len() + 3; // skip `"key":[`
    // find actual `[`
    let abs = s[bracket_pos..].find('[')? + bracket_pos;
    find_matching_bracket(s, abs)
}

/// Starting at `pos` which must point at `[`, return the slice `[...]`.
fn find_matching_bracket(s: &str, pos: usize) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.get(pos) != Some(&b'[') {
        return None;
    }
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes[pos..].iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if in_string {
            if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[pos..=pos + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Starting at `pos` which must point at `{`, return the slice `{...}`.
fn find_matching_brace(s: &str, pos: usize) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.get(pos) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes[pos..].iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if in_string {
            if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[pos..=pos + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a JSON array `[obj, obj, …]` into the raw object slices.
fn split_objects(array: &str) -> Vec<&str> {
    let inner = array.trim();
    let inner = if inner.starts_with('[') && inner.ends_with(']') {
        &inner[1..inner.len() - 1]
    } else {
        inner
    };
    let mut objects: Vec<&str> = Vec::new();
    let mut pos = 0;
    let bytes = inner.as_bytes();
    while pos < bytes.len() {
        // skip whitespace and commas
        if bytes[pos] == b' ' || bytes[pos] == b',' || bytes[pos] == b'\n' || bytes[pos] == b'\t' {
            pos += 1;
            continue;
        }
        if bytes[pos] == b'{' {
            if let Some(obj) = find_matching_brace(inner, pos) {
                objects.push(obj);
                pos += obj.len();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    objects
}

fn extract_messages(s: &str) -> Result<Vec<HandoffMessage>, String> {
    let array_raw = match extract_array_raw(s, "messages") {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };
    let mut messages = Vec::new();
    for obj in split_objects(array_raw) {
        let role_str = extract_str_field(obj, "role").unwrap_or_else(|| "user".into());
        let role = role_from_str(&role_str);
        let parts = extract_parts(obj)?;
        let metadata = extract_map_field(obj, "metadata");
        messages.push(HandoffMessage {
            role,
            parts,
            metadata,
        });
    }
    Ok(messages)
}

fn extract_parts(msg: &str) -> Result<Vec<ContentPart>, String> {
    let array_raw = match extract_array_raw(msg, "parts") {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };
    let mut parts = Vec::new();
    for obj in split_objects(array_raw) {
        let kind_str = extract_str_field(obj, "kind").unwrap_or_else(|| "text".into());
        let kind = kind_from_str(&kind_str);
        parts.push(ContentPart {
            kind,
            text: extract_optional_str_field(obj, "text"),
            tool_call_id: extract_optional_str_field(obj, "tool_call_id"),
            tool_name: extract_optional_str_field(obj, "tool_name"),
            tool_args: extract_optional_str_field(obj, "tool_args"),
            tool_result: extract_optional_str_field(obj, "tool_result"),
        });
    }
    Ok(parts)
}

fn extract_tools(s: &str) -> Result<Vec<ToolDefinition>, String> {
    let array_raw = match extract_array_raw(s, "tools") {
        Some(a) => a,
        None => return Ok(Vec::new()),
    };
    let mut tools = Vec::new();
    for obj in split_objects(array_raw) {
        let name = extract_str_field(obj, "name").unwrap_or_default();
        let description = extract_str_field(obj, "description").unwrap_or_default();
        let parameters_json = extract_str_field(obj, "parameters_json").unwrap_or_default();
        tools.push(ToolDefinition {
            name,
            description,
            parameters_json,
        });
    }
    Ok(tools)
}

fn extract_map_field(s: &str, key: &str) -> HashMap<String, String> {
    let needle = format!("\"{}\":{{", key);
    let start = match s.find(needle.as_str()) {
        Some(p) => p + needle.len() - 1,
        None => return HashMap::new(),
    };
    let obj_raw = match find_matching_brace(s, start) {
        Some(o) => o,
        None => return HashMap::new(),
    };
    // Parse `{"k":"v","k2":"v2"}` — simple flat string-string map
    let inner = &obj_raw[1..obj_raw.len() - 1];
    let mut map = HashMap::new();
    let mut rest = inner;
    while !rest.trim().is_empty() {
        rest = rest.trim_start_matches([' ', ',', '\n', '\t'].as_ref());
        if rest.is_empty() {
            break;
        }
        let k = match parse_json_string(rest) {
            Some(k) => k,
            None => break,
        };
        // advance past the key string in `rest`
        rest = advance_past_string(rest);
        rest = rest.trim_start();
        if rest.starts_with(':') {
            rest = &rest[1..];
        }
        rest = rest.trim_start();
        let v = match parse_json_string(rest) {
            Some(v) => v,
            None => break,
        };
        rest = advance_past_string(rest);
        map.insert(k, v);
    }
    map
}

/// Advance `s` past the leading JSON string literal (including the quotes).
fn advance_past_string(s: &str) -> &str {
    let s = s.trim_start();
    if !s.starts_with('"') {
        return s;
    }
    let s = &s[1..];
    let mut chars = s.char_indices();
    let mut escape = false;
    loop {
        match chars.next() {
            None => return "",
            Some((i, '\\')) if !escape => {
                escape = true;
                let _ = i;
            }
            Some((i, '"')) if !escape => return &s[i + 1..],
            Some(_) => escape = false,
        }
    }
}

// ---------------------------------------------------------------------------
// Timestamp helper (no std::time dependency in no_std builds)
// ---------------------------------------------------------------------------

fn timestamp_now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build a small context with 3 messages and 1 tool.
    fn sample_context() -> HandoffContext {
        let mut ctx = HandoffContext::new("claude")
            .with_system("You are a helpful coding assistant.");
        ctx.push_message(HandoffMessage::user("Explain ownership in Rust."));
        ctx.push_message(HandoffMessage::assistant("Ownership is a set of rules…"));
        ctx.push_message(HandoffMessage::user("Give me an example."));
        ctx.push_tool(ToolDefinition::new(
            "run_code",
            "Execute code in a sandbox.",
            r#"{"type":"object","properties":{"code":{"type":"string"}}}"#,
        ));
        ctx
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let ctx = sample_context();
        let json = ctx.serialize().expect("serialize failed");
        let restored = HandoffContext::deserialize(&json).expect("deserialize failed");

        assert_eq!(restored.source_provider, "claude");
        assert!(restored.target_provider.is_none());
        assert_eq!(
            restored.system_prompt.as_deref(),
            Some("You are a helpful coding assistant.")
        );
        assert_eq!(restored.message_count(), 3);
        assert_eq!(restored.tools.len(), 1);
        assert_eq!(restored.tools[0].name, "run_code");
    }

    #[test]
    fn test_serialize_deserialize_with_target() {
        let ctx = sample_context().for_provider("haiku");
        let json = ctx.serialize().expect("serialize failed");
        let restored = HandoffContext::deserialize(&json).expect("deserialize failed");
        assert_eq!(restored.target_provider.as_deref(), Some("haiku"));
    }

    #[test]
    fn test_trim_to_token_budget_drops_oldest() {
        let mut ctx = HandoffContext::new("openai").with_system("sys");
        // Push 10 short messages (~4 tokens each)
        for i in 0..10 {
            ctx.push_message(HandoffMessage::user(format!("msg{}", i)));
        }
        let full_count = ctx.message_count();
        // Very small budget — should drop messages
        let trimmed = ctx.trim_to_token_budget(1);
        assert!(
            trimmed.message_count() < full_count,
            "expected messages to be trimmed, had {}",
            trimmed.message_count()
        );
        // If any messages remain, they should be the newest ones (end of vec)
        if trimmed.message_count() > 0 {
            let last = trimmed.messages.last().unwrap();
            assert!(last.text_content().starts_with("msg9"));
        }
    }

    #[test]
    fn test_trim_does_not_drop_below_zero() {
        let ctx = HandoffContext::new("openai").with_system("short");
        // Even with budget=0 the function should not panic
        let trimmed = ctx.trim_to_token_budget(0);
        assert_eq!(trimmed.message_count(), 0);
    }

    #[test]
    fn test_for_provider_clone() {
        let ctx = sample_context();
        let routed = ctx.for_provider("gemini");
        assert_eq!(routed.target_provider.as_deref(), Some("gemini"));
        // Original is unchanged
        assert!(ctx.target_provider.is_none());
        // Content preserved
        assert_eq!(routed.message_count(), ctx.message_count());
        assert_eq!(routed.tools.len(), ctx.tools.len());
    }

    #[test]
    fn test_summary_format() {
        let ctx = sample_context();
        let s = ctx.summary();
        assert!(s.contains("3 messages"), "summary: {}", s);
        assert!(s.contains("1 tools"), "summary: {}", s);
        assert!(s.contains("tokens"), "summary: {}", s);
    }

    #[test]
    fn test_last_user_message() {
        let ctx = sample_context();
        let last = ctx.last_user_message().unwrap();
        assert_eq!(last.text_content(), "Give me an example.");
    }

    #[test]
    fn test_has_tool_calls() {
        let mut msg = HandoffMessage::assistant("Let me run that.");
        assert!(!msg.has_tool_calls());
        msg.parts.push(ContentPart::tool_call("call-1", "run_code", r#"{"code":"1+1"}"#));
        assert!(msg.has_tool_calls());
    }

    #[test]
    fn test_tool_result_message() {
        let msg = HandoffMessage::tool_result("call-1", "2");
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.parts[0].tool_result.as_deref(), Some("2"));
    }

    #[test]
    fn test_handoff_history_record_and_providers_used() {
        let mut history = HandoffHistory::new();
        assert_eq!(history.count(), 0);
        assert!(history.last().is_none());

        history.record("claude-sonnet", "claude-haiku", HandoffReason::CostRouting, 8);
        history.record("claude-haiku", "gemini-flash", HandoffReason::CapabilityGap, 14);

        assert_eq!(history.count(), 2);
        let last = history.last().unwrap();
        assert_eq!(last.from_provider, "claude-haiku");
        assert_eq!(last.to_provider, "gemini-flash");
        assert_eq!(last.reason, HandoffReason::CapabilityGap);
        assert_eq!(last.message_count_at_handoff, 14);

        let providers = history.providers_used();
        assert_eq!(providers, vec!["claude-sonnet", "claude-haiku", "gemini-flash"]);
    }

    #[test]
    fn test_providers_used_deduplicates() {
        let mut history = HandoffHistory::new();
        history.record("a", "b", HandoffReason::Fallback, 0);
        history.record("b", "a", HandoffReason::UserRequested, 0);
        let providers = history.providers_used();
        assert_eq!(providers, vec!["a", "b"]);
    }

    #[test]
    fn test_empty_context_serialize_deserialize() {
        let ctx = HandoffContext::new("groq");
        let json = ctx.serialize().expect("serialize failed");
        let restored = HandoffContext::deserialize(&json).expect("deserialize failed");
        assert_eq!(restored.source_provider, "groq");
        assert_eq!(restored.message_count(), 0);
        assert!(restored.tools.is_empty());
        assert!(restored.system_prompt.is_none());
    }

    #[test]
    fn test_token_estimate_rough() {
        let mut ctx = HandoffContext::new("openai");
        // 400 chars of system prompt → ~100 tokens
        ctx.system_prompt = Some("x".repeat(400));
        let est = ctx.token_estimate();
        assert_eq!(est, 100);
    }

    #[test]
    fn test_text_content_concatenates_parts() {
        let mut msg = HandoffMessage::user("Hello");
        msg.parts.push(ContentPart::text(" World"));
        assert_eq!(msg.text_content(), "Hello\n World");
    }

    #[test]
    fn test_special_chars_in_serialize_roundtrip() {
        let mut ctx = HandoffContext::new("claude");
        ctx.push_message(HandoffMessage::user(r#"Say "hello" and \backslash"#));
        let json = ctx.serialize().expect("serialize failed");
        let restored = HandoffContext::deserialize(&json).expect("deserialize failed");
        let text = restored.messages[0].text_content();
        assert_eq!(text, r#"Say "hello" and \backslash"#);
    }
}
