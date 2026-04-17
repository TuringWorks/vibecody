//! Bidirectional stdin/stdout JSONL RPC for embedding VibeCLI.
//!
//! Each message is a single UTF-8 line terminated with `\n` (LF only —
//! never `\r\n`).  This avoids readline splitting on Unicode paragraph/line
//! separators that appear inside JSON string values.
//!
//! Pi-mono gap bridge: Phase C4.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Message-type enums
// ---------------------------------------------------------------------------

/// Inbound message types (caller → VibeCLI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundType {
    /// `{ "type": "send_message", "content": "...", "role": "user" }`
    SendMessage,
    /// `{ "type": "interrupt" }`
    Interrupt,
    /// `{ "type": "set_config", "key": "...", "value": "..." }`
    SetConfig,
    /// `{ "type": "shutdown" }`
    Shutdown,
    /// `{ "type": "ping", "id": "..." }`
    Ping,
}

impl InboundType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InboundType::SendMessage => "send_message",
            InboundType::Interrupt => "interrupt",
            InboundType::SetConfig => "set_config",
            InboundType::Shutdown => "shutdown",
            InboundType::Ping => "ping",
        }
    }
}

/// Outbound message types (VibeCLI → caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundType {
    /// `{ "type": "agent_start" }`
    AgentStart,
    /// `{ "type": "agent_end" }`
    AgentEnd,
    /// `{ "type": "token_delta", "text": "..." }`
    TokenDelta,
    /// `{ "type": "tool_call", "name": "...", "args": "..." }`
    ToolCall,
    /// `{ "type": "tool_result", "name": "...", "output": "...", "exit_code": 0 }`
    ToolResult,
    /// `{ "type": "error", "message": "..." }`
    Error,
    /// `{ "type": "pong", "id": "..." }`
    Pong,
    /// `{ "type": "token_usage", "input": N, "output": N, "cost_usd": F }`
    TokenUsage,
    /// `{ "type": "session_start", "session_id": "..." }`
    SessionStart,
    /// `{ "type": "session_end", "session_id": "..." }`
    SessionEnd,
}

impl OutboundType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutboundType::AgentStart => "agent_start",
            OutboundType::AgentEnd => "agent_end",
            OutboundType::TokenDelta => "token_delta",
            OutboundType::ToolCall => "tool_call",
            OutboundType::ToolResult => "tool_result",
            OutboundType::Error => "error",
            OutboundType::Pong => "pong",
            OutboundType::TokenUsage => "token_usage",
            OutboundType::SessionStart => "session_start",
            OutboundType::SessionEnd => "session_end",
        }
    }
}

// ---------------------------------------------------------------------------
// RpcFrame
// ---------------------------------------------------------------------------

/// A raw RPC frame — one JSONL line.
#[derive(Debug, Clone)]
pub struct RpcFrame {
    pub msg_type: String,
    pub payload: HashMap<String, serde_json::Value>,
}

impl RpcFrame {
    /// Create a new frame with the given type and an empty payload.
    pub fn new(msg_type: impl Into<String>) -> Self {
        Self {
            msg_type: msg_type.into(),
            payload: HashMap::new(),
        }
    }

    /// Builder: add a key/value pair to the payload.
    pub fn with(mut self, key: &str, value: serde_json::Value) -> Self {
        self.payload.insert(key.to_string(), value);
        self
    }

    /// Get a string field from the payload.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.payload.get(key)?.as_str()
    }

    /// Get a u64 field from the payload.
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.payload.get(key)?.as_u64()
    }

    /// Serialize to a single JSON line terminated with `\n` (LF only, never CRLF).
    pub fn to_jsonl(&self) -> String {
        let mut map = serde_json::Map::new();
        map.insert(
            "type".to_string(),
            serde_json::Value::String(self.msg_type.clone()),
        );
        for (k, v) in &self.payload {
            map.insert(k.clone(), v.clone());
        }
        let mut s = serde_json::to_string(&serde_json::Value::Object(map))
            .unwrap_or_else(|_| r#"{"type":"error","message":"serialize_failed"}"#.to_string());
        // Enforce LF-only: strip any trailing \r before appending \n.
        if s.ends_with('\r') {
            s.pop();
        }
        s.push('\n');
        s
    }

    /// Parse from a single line. Returns `Err` if the input is not valid JSON
    /// or if the `"type"` field is absent.
    pub fn from_line(line: &str) -> Result<Self, String> {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| format!("json parse error: {e}"))?;
        let obj = value
            .as_object()
            .ok_or_else(|| "expected JSON object".to_string())?;
        let msg_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing \"type\" field".to_string())?
            .to_string();
        let payload: HashMap<String, serde_json::Value> = obj
            .iter()
            .filter(|(k, _)| k.as_str() != "type")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(Self { msg_type, payload })
    }

    /// Return `true` if this frame's type matches the given inbound type.
    pub fn is_inbound_type(&self, t: InboundType) -> bool {
        self.msg_type == t.as_str()
    }

    /// Return `true` if this frame's type matches the given outbound type.
    pub fn is_outbound_type(&self, t: OutboundType) -> bool {
        self.msg_type == t.as_str()
    }
}

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

impl RpcFrame {
    /// `{ "type": "token_delta", "text": "..." }`
    pub fn token_delta(text: &str) -> Self {
        Self::new("token_delta").with("text", serde_json::Value::String(text.to_string()))
    }

    /// `{ "type": "tool_call", "name": "...", "args": "..." }`
    pub fn tool_call(name: &str, args: &str) -> Self {
        Self::new("tool_call")
            .with("name", serde_json::Value::String(name.to_string()))
            .with("args", serde_json::Value::String(args.to_string()))
    }

    /// `{ "type": "tool_result", "name": "...", "output": "...", "exit_code": N }`
    pub fn tool_result(name: &str, output: &str, exit_code: i32) -> Self {
        Self::new("tool_result")
            .with("name", serde_json::Value::String(name.to_string()))
            .with("output", serde_json::Value::String(output.to_string()))
            .with("exit_code", serde_json::Value::Number(exit_code.into()))
    }

    /// `{ "type": "error", "message": "..." }`
    pub fn error(message: &str) -> Self {
        Self::new("error").with("message", serde_json::Value::String(message.to_string()))
    }

    /// `{ "type": "pong", "id": "..." }`
    pub fn pong(id: &str) -> Self {
        Self::new("pong").with("id", serde_json::Value::String(id.to_string()))
    }

    /// `{ "type": "agent_start" }`
    pub fn agent_start() -> Self {
        Self::new("agent_start")
    }

    /// `{ "type": "agent_end" }`
    pub fn agent_end() -> Self {
        Self::new("agent_end")
    }

    /// `{ "type": "token_usage", "input": N, "output": N, "cost_usd": F }`
    pub fn token_usage(input: u64, output: u64, cost_usd: f64) -> Self {
        Self::new("token_usage")
            .with("input", serde_json::Value::Number(input.into()))
            .with("output", serde_json::Value::Number(output.into()))
            .with(
                "cost_usd",
                serde_json::Number::from_f64(cost_usd)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::String(cost_usd.to_string())),
            )
    }
}

// ---------------------------------------------------------------------------
// RpcReader
// ---------------------------------------------------------------------------

/// Reads JSONL frames from a `BufRead` source (e.g., `stdin`).
pub struct RpcReader<R: std::io::BufRead> {
    reader: R,
}

impl<R: std::io::BufRead> RpcReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Read the next frame.  Returns `None` on EOF, `Some(Err(...))` on a
    /// malformed line, `Some(Ok(frame))` on success.
    pub fn next_frame(&mut self) -> Option<Result<RpcFrame, String>> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    // Skip blank lines and try again.
                    return self.next_frame();
                }
                Some(RpcFrame::from_line(trimmed))
            }
            Err(e) => Some(Err(format!("io error: {e}"))),
        }
    }

    /// Drain all remaining frames, silently dropping errors.
    pub fn collect_frames(&mut self) -> Vec<RpcFrame> {
        let mut out = Vec::new();
        while let Some(result) = self.next_frame() {
            if let Ok(frame) = result {
                out.push(frame);
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// RpcWriter
// ---------------------------------------------------------------------------

/// Writes JSONL frames to a `Write` sink (e.g., `stdout`).
pub struct RpcWriter<W: std::io::Write> {
    writer: W,
}

impl<W: std::io::Write> RpcWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Serialize `frame` as a single LF-terminated JSONL line and write it.
    pub fn send(&mut self, frame: &RpcFrame) -> Result<(), String> {
        let line = frame.to_jsonl();
        self.writer
            .write_all(line.as_bytes())
            .map_err(|e| format!("write error: {e}"))
    }

    /// Flush the underlying writer.
    pub fn flush(&mut self) -> Result<(), String> {
        self.writer
            .flush()
            .map_err(|e| format!("flush error: {e}"))
    }
}

// ---------------------------------------------------------------------------
// MemoryTransport
// ---------------------------------------------------------------------------

/// In-memory bidirectional transport for testing.
///
/// Inbound: lines pushed via `push_inbound` are consumed by an `RpcReader`
/// built from this transport.  Outbound: frames written by an `RpcWriter`
/// built from this transport are captured and retrieved via `pop_outbound`.
#[derive(Debug)]
pub struct MemoryTransport {
    pub inbound: Mutex<VecDeque<String>>,
    pub outbound: Mutex<Vec<String>>,
}

impl Default for MemoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTransport {
    pub fn new() -> Self {
        Self {
            inbound: Mutex::new(VecDeque::new()),
            outbound: Mutex::new(Vec::new()),
        }
    }

    /// Enqueue a frame as a JSONL line for reading.
    pub fn push_inbound(&self, frame: &RpcFrame) {
        let line = frame.to_jsonl();
        self.inbound.lock().unwrap().push_back(line);
    }

    /// Drain and parse all captured outbound lines into frames (errors dropped).
    pub fn pop_outbound(&self) -> Vec<RpcFrame> {
        let lines: Vec<String> = {
            let mut guard = self.outbound.lock().unwrap();
            guard.drain(..).collect()
        };
        lines
            .iter()
            .filter_map(|l| RpcFrame::from_line(l).ok())
            .collect()
    }

    /// Number of captured outbound lines (without consuming them).
    pub fn outbound_count(&self) -> usize {
        self.outbound.lock().unwrap().len()
    }

    /// Build an `RpcReader` backed by the inbound queue.
    pub fn reader(&self) -> RpcReader<MemoryBufRead<'_>> {
        RpcReader::new(MemoryBufRead {
            queue: &self.inbound,
            current: String::new(),
            pos: 0,
        })
    }

    /// Build an `RpcWriter` backed by the outbound capture buffer.
    pub fn writer(&self) -> RpcWriter<MemoryWrite<'_>> {
        RpcWriter::new(MemoryWrite {
            buf: &self.outbound,
            line_buf: String::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Helper I/O adapters for MemoryTransport
// ---------------------------------------------------------------------------

/// `BufRead` implementation that reads from the inbound `Mutex<VecDeque>`.
pub struct MemoryBufRead<'a> {
    queue: &'a Mutex<VecDeque<String>>,
    current: String,
    pos: usize,
}

impl<'a> std::io::Read for MemoryBufRead<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Refill from the queue when the current line is exhausted.
        while self.pos >= self.current.len() {
            let next = self.queue.lock().unwrap().pop_front();
            match next {
                Some(line) => {
                    self.current = line;
                    self.pos = 0;
                }
                None => return Ok(0), // EOF
            }
        }
        let remaining = &self.current.as_bytes()[self.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        Ok(n)
    }
}

impl<'a> std::io::BufRead for MemoryBufRead<'a> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.pos >= self.current.len() {
            let next = self.queue.lock().unwrap().pop_front();
            match next {
                Some(line) => {
                    self.current = line;
                    self.pos = 0;
                }
                None => {
                    self.current = String::new();
                    self.pos = 0;
                    return Ok(&[]);
                }
            }
        }
        Ok(&self.current.as_bytes()[self.pos..])
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }
}

/// `Write` implementation that appends complete lines to the outbound buffer.
pub struct MemoryWrite<'a> {
    buf: &'a Mutex<Vec<String>>,
    line_buf: String,
}

impl<'a> std::io::Write for MemoryWrite<'a> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let s = std::str::from_utf8(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.line_buf.push_str(s);
        // Flush complete lines into the shared buffer.
        while let Some(pos) = self.line_buf.find('\n') {
            let line = self.line_buf[..=pos].to_string();
            self.buf.lock().unwrap().push(line);
            self.line_buf = self.line_buf[pos + 1..].to_string();
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // If there is a partial line (no trailing newline), flush it anyway.
        if !self.line_buf.is_empty() {
            let line = std::mem::take(&mut self.line_buf);
            self.buf.lock().unwrap().push(line);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RpcModeConfig
// ---------------------------------------------------------------------------

/// Configuration for an RPC mode session.
#[derive(Debug, Clone)]
pub struct RpcModeConfig {
    /// Unique identifier for this session (emitted in `session_start` / `session_end`).
    pub session_id: String,
    /// Emit incremental `token_delta` frames during generation.
    pub emit_token_deltas: bool,
    /// Emit `tool_call` and `tool_result` frames.
    pub emit_tool_events: bool,
    /// Emit a `token_usage` frame at the end of each turn.
    pub emit_usage: bool,
    /// Enforce LF-only line endings (recommended: `true`).
    pub strict_lf: bool,
}

impl Default for RpcModeConfig {
    fn default() -> Self {
        Self {
            session_id: "default".to_string(),
            emit_token_deltas: true,
            emit_tool_events: true,
            emit_usage: true,
            strict_lf: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- to_jsonl LF framing ---

    #[test]
    fn to_jsonl_ends_with_lf_not_crlf() {
        let frame = RpcFrame::token_delta("hello");
        let line = frame.to_jsonl();
        assert!(line.ends_with('\n'), "must end with LF");
        assert!(!line.ends_with("\r\n"), "must NOT end with CRLF");
    }

    #[test]
    fn to_jsonl_single_newline_at_end() {
        let frame = RpcFrame::agent_start();
        let line = frame.to_jsonl();
        assert_eq!(line.chars().filter(|&c| c == '\n').count(), 1);
    }

    // --- from_line ---

    #[test]
    fn from_line_parses_type_and_payload() {
        let line = r#"{"type":"token_delta","text":"hello world"}"#;
        let frame = RpcFrame::from_line(line).unwrap();
        assert_eq!(frame.msg_type, "token_delta");
        assert_eq!(frame.get_str("text"), Some("hello world"));
    }

    #[test]
    fn from_line_strips_trailing_newline() {
        let line = "{\"type\":\"ping\",\"id\":\"abc\"}\n";
        let frame = RpcFrame::from_line(line).unwrap();
        assert_eq!(frame.msg_type, "ping");
        assert_eq!(frame.get_str("id"), Some("abc"));
    }

    #[test]
    fn from_line_strips_crlf() {
        let line = "{\"type\":\"ping\",\"id\":\"x\"}\r\n";
        let frame = RpcFrame::from_line(line).unwrap();
        assert_eq!(frame.msg_type, "ping");
    }

    #[test]
    fn from_line_missing_type_returns_err() {
        let line = r#"{"content":"no type here"}"#;
        let result = RpcFrame::from_line(line);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("missing") || msg.contains("type"), "error: {msg}");
    }

    #[test]
    fn from_line_invalid_json_returns_err() {
        let result = RpcFrame::from_line("not json at all");
        assert!(result.is_err());
    }

    // --- convenience constructors ---

    #[test]
    fn token_delta_frame_roundtrip() {
        let frame = RpcFrame::token_delta("world");
        let line = frame.to_jsonl();
        let parsed = RpcFrame::from_line(&line).unwrap();
        assert!(parsed.is_outbound_type(OutboundType::TokenDelta));
        assert_eq!(parsed.get_str("text"), Some("world"));
    }

    #[test]
    fn tool_call_frame_has_name_and_args() {
        let frame = RpcFrame::tool_call("bash", "{\"cmd\":\"ls\"}");
        assert!(frame.is_outbound_type(OutboundType::ToolCall));
        assert_eq!(frame.get_str("name"), Some("bash"));
        assert_eq!(frame.get_str("args"), Some("{\"cmd\":\"ls\"}"));
    }

    #[test]
    fn tool_result_frame_has_exit_code() {
        let frame = RpcFrame::tool_result("bash", "file.txt\n", 0);
        assert_eq!(frame.get_str("name"), Some("bash"));
        assert_eq!(frame.get_str("output"), Some("file.txt\n"));
        assert_eq!(frame.get_u64("exit_code"), Some(0));
    }

    #[test]
    fn error_frame_has_message() {
        let frame = RpcFrame::error("something went wrong");
        assert!(frame.is_outbound_type(OutboundType::Error));
        assert_eq!(frame.get_str("message"), Some("something went wrong"));
    }

    #[test]
    fn pong_frame_has_id() {
        let frame = RpcFrame::pong("req-42");
        assert!(frame.is_outbound_type(OutboundType::Pong));
        assert_eq!(frame.get_str("id"), Some("req-42"));
    }

    #[test]
    fn token_usage_frame() {
        let frame = RpcFrame::token_usage(1024, 256, 0.003);
        assert!(frame.is_outbound_type(OutboundType::TokenUsage));
        assert_eq!(frame.get_u64("input"), Some(1024));
        assert_eq!(frame.get_u64("output"), Some(256));
    }

    #[test]
    fn agent_start_and_end_types() {
        let s = RpcFrame::agent_start();
        let e = RpcFrame::agent_end();
        assert!(s.is_outbound_type(OutboundType::AgentStart));
        assert!(e.is_outbound_type(OutboundType::AgentEnd));
    }

    // --- is_inbound_type ---

    #[test]
    fn is_inbound_type_ping() {
        let frame = RpcFrame::new("ping").with("id", serde_json::Value::String("1".into()));
        assert!(frame.is_inbound_type(InboundType::Ping));
        assert!(!frame.is_inbound_type(InboundType::Shutdown));
    }

    // --- MemoryTransport ---

    #[test]
    fn memory_transport_push_pop_roundtrip() {
        let transport = MemoryTransport::new();
        let frame = RpcFrame::token_delta("stream chunk");
        transport.push_inbound(&frame);

        let mut reader = transport.reader();
        let received = reader.next_frame().unwrap().unwrap();
        assert_eq!(received.msg_type, "token_delta");
        assert_eq!(received.get_str("text"), Some("stream chunk"));
    }

    #[test]
    fn memory_transport_outbound_capture() {
        let transport = MemoryTransport::new();
        {
            let mut writer = transport.writer();
            writer.send(&RpcFrame::pong("p1")).unwrap();
            writer.send(&RpcFrame::agent_end()).unwrap();
            writer.flush().unwrap();
        }
        assert_eq!(transport.outbound_count(), 2);
        let frames = transport.pop_outbound();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].msg_type, "pong");
        assert_eq!(frames[1].msg_type, "agent_end");
    }

    #[test]
    fn memory_transport_outbound_count_resets_after_pop() {
        let transport = MemoryTransport::new();
        {
            let mut writer = transport.writer();
            writer.send(&RpcFrame::agent_start()).unwrap();
            writer.flush().unwrap();
        }
        assert_eq!(transport.outbound_count(), 1);
        transport.pop_outbound();
        assert_eq!(transport.outbound_count(), 0);
    }

    // --- RpcReader ---

    #[test]
    fn rpc_reader_reads_multiple_frames() {
        let input = concat!(
            "{\"type\":\"send_message\",\"content\":\"hello\"}\n",
            "{\"type\":\"interrupt\"}\n",
            "{\"type\":\"shutdown\"}\n",
        );
        let cursor = std::io::BufReader::new(std::io::Cursor::new(input));
        let mut reader = RpcReader::new(cursor);
        let frames = reader.collect_frames();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].msg_type, "send_message");
        assert_eq!(frames[1].msg_type, "interrupt");
        assert_eq!(frames[2].msg_type, "shutdown");
    }

    #[test]
    fn rpc_reader_skips_blank_lines() {
        let input = "\n{\"type\":\"ping\",\"id\":\"1\"}\n\n{\"type\":\"shutdown\"}\n";
        let cursor = std::io::BufReader::new(std::io::Cursor::new(input));
        let mut reader = RpcReader::new(cursor);
        let frames = reader.collect_frames();
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn rpc_reader_returns_none_on_eof() {
        let cursor = std::io::BufReader::new(std::io::Cursor::new(""));
        let mut reader = RpcReader::new(cursor);
        assert!(reader.next_frame().is_none());
    }

    // --- RpcWriter ---

    #[test]
    fn rpc_writer_captures_output() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = RpcWriter::new(buf);
        writer.send(&RpcFrame::pong("id-1")).unwrap();
        writer.flush().unwrap();
        let raw = writer.writer;
        let s = std::str::from_utf8(&raw).unwrap();
        assert!(s.contains("\"pong\"") || s.contains("\"type\":\"pong\""));
        assert!(s.ends_with('\n'));
        assert!(!s.ends_with("\r\n"));
    }

    // --- RpcModeConfig defaults ---

    #[test]
    fn rpc_mode_config_default_strict_lf() {
        let cfg = RpcModeConfig::default();
        assert!(cfg.strict_lf);
        assert!(cfg.emit_token_deltas);
        assert!(cfg.emit_tool_events);
        assert!(cfg.emit_usage);
    }
}
