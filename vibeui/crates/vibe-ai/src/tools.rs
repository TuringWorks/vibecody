//! Universal prompt-based tool framework.
//!
//! Works with every LLM provider (Ollama, Claude, OpenAI, Gemini, Grok) by injecting
//! tool definitions into the system prompt and parsing `<tool_call>` XML blocks from
//! model output — no native function-calling API required.

use regex::Regex;
use serde::{Deserialize, Serialize};

// ── System Prompt ─────────────────────────────────────────────────────────────

/// System prompt fragment that teaches the model how to call tools.
/// Prepended to every agent conversation.
pub const TOOL_SYSTEM_PROMPT: &str = r#"
You are VibeCLI, an autonomous coding agent running in the user's terminal.

## Tool Use

To use a tool, output ONLY a single `<tool_call>` block — no other text on the same response:

```
<tool_call name="TOOL_NAME">
<param_name>param value</param_name>
</tool_call>
```

After each tool result is shown to you, decide the next step. Never call more than
one tool per response. When the task is fully complete, call `task_complete`.

## Available Tools

### read_file
Read the contents of a file at the given path.
```
<tool_call name="read_file">
<path>/path/to/file.rs</path>
</tool_call>
```

### write_file
Write (create or overwrite) content to a file. The content must be the complete file.
```
<tool_call name="write_file">
<path>/path/to/file.rs</path>
<content>
fn main() { println!("Hello"); }
</content>
</tool_call>
```

### apply_patch
Apply a unified diff patch to modify an existing file.
```
<tool_call name="apply_patch">
<path>/path/to/file.rs</path>
<patch>
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,4 @@
 fn main() {
-    println!("Hello");
+    println!("Hello, World!");
 }
</patch>
</tool_call>
```

### bash
Execute a shell command and return stdout + stderr.
```
<tool_call name="bash">
<command>cargo test 2>&1 | head -50</command>
</tool_call>
```

### search_files
Search for files matching a pattern or containing specific text.
```
<tool_call name="search_files">
<query>search term or regex</query>
<glob>*.rs</glob>
</tool_call>
```

### list_directory
List all files and directories at the given path.
```
<tool_call name="list_directory">
<path>.</path>
</tool_call>
```

### web_search
Search the web for current information using DuckDuckGo. No API key required.
```
<tool_call name="web_search">
<query>rust async await tutorial</query>
<num_results>5</num_results>
</tool_call>
```

### fetch_url
Fetch and extract the text content of a web page.
```
<tool_call name="fetch_url">
<url>https://doc.rust-lang.org/book/ch01-00-getting-started.html</url>
</tool_call>
```

### task_complete
Call this when the task is fully done. Provide a summary of what was accomplished.
```
<tool_call name="task_complete">
<summary>Created hello.rs with a main function that prints Hello World.</summary>
</tool_call>
```

### spawn_agent
Delegate an independent sub-task to a child agent. The child runs with the same tools and
workspace. Use this to parallelize work or isolate complex sub-problems.
```
<tool_call name="spawn_agent">
<task>Write unit tests for src/utils.rs and verify they pass with cargo test.</task>
<max_steps>10</max_steps>
</tool_call>
```

## Important Rules
- Output ONLY the `<tool_call>` block when calling a tool — no prose before or after.
- After a tool result, you may think briefly then call the next tool or conclude.
- Never repeat a failed tool call identically — adjust the approach.
- Prefer reading files before writing them to understand existing structure.
- Keep bash commands focused and safe; prefer read-only operations first.
"#;

// ── ToolCall ─────────────────────────────────────────────────────────────────

/// A parsed tool invocation from model output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCall {
    ReadFile {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    ApplyPatch {
        path: String,
        patch: String,
    },
    Bash {
        command: String,
    },
    SearchFiles {
        query: String,
        glob: Option<String>,
    },
    ListDirectory {
        path: String,
    },
    /// Search the web using DuckDuckGo (no API key required).
    WebSearch {
        query: String,
        num_results: usize,
    },
    /// Fetch the text content of a URL.
    FetchUrl {
        url: String,
    },
    TaskComplete {
        summary: String,
    },
    /// Spawn a sub-agent to complete a sub-task autonomously.
    /// The sub-agent runs with the same tools and approval policy as the parent.
    /// Use this to delegate independent work streams or specialized tasks.
    SpawnAgent {
        /// The task or question for the sub-agent to complete.
        task: String,
        /// Maximum number of steps the sub-agent can take (default: 10).
        max_steps: Option<usize>,
    },
}

impl ToolCall {
    /// Human-readable name of this tool.
    pub fn name(&self) -> &'static str {
        match self {
            ToolCall::ReadFile { .. } => "read_file",
            ToolCall::WriteFile { .. } => "write_file",
            ToolCall::ApplyPatch { .. } => "apply_patch",
            ToolCall::Bash { .. } => "bash",
            ToolCall::SearchFiles { .. } => "search_files",
            ToolCall::ListDirectory { .. } => "list_directory",
            ToolCall::WebSearch { .. } => "web_search",
            ToolCall::FetchUrl { .. } => "fetch_url",
            ToolCall::TaskComplete { .. } => "task_complete",
            ToolCall::SpawnAgent { .. } => "spawn_agent",
        }
    }

    /// Short human-readable summary of this call (for UI display).
    pub fn summary(&self) -> String {
        match self {
            ToolCall::ReadFile { path } => format!("read_file({})", path),
            ToolCall::WriteFile { path, content } => {
                let lines = content.lines().count();
                format!("write_file({}, {} lines)", path, lines)
            }
            ToolCall::ApplyPatch { path, patch } => {
                let hunks = patch.lines().filter(|l| l.starts_with("@@")).count();
                format!("apply_patch({}, {} hunks)", path, hunks)
            }
            ToolCall::Bash { command } => {
                let cmd = if command.len() > 60 {
                    format!("{}…", &command[..60])
                } else {
                    command.clone()
                };
                format!("bash({})", cmd)
            }
            ToolCall::SearchFiles { query, glob } => match glob {
                Some(g) => format!("search_files({:?} in {})", query, g),
                None => format!("search_files({:?})", query),
            },
            ToolCall::ListDirectory { path } => format!("list_directory({})", path),
            ToolCall::WebSearch { query, num_results } => {
                format!("web_search({:?}, {})", query, num_results)
            }
            ToolCall::FetchUrl { url } => format!("fetch_url({})", url),
            ToolCall::TaskComplete { summary } => {
                let short = if summary.len() > 60 {
                    format!("{}…", &summary[..60])
                } else {
                    summary.clone()
                };
                format!("task_complete: {}", short)
            }
            ToolCall::SpawnAgent { task, max_steps } => {
                let short = if task.len() > 60 { format!("{}…", &task[..60]) } else { task.clone() };
                format!("spawn_agent(task={:?}, max_steps={})", short, max_steps.unwrap_or(10))
            }
        }
    }

    /// Returns true if this is a destructive / risky operation.
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            ToolCall::Bash { .. } | ToolCall::WriteFile { .. } | ToolCall::ApplyPatch { .. }
                | ToolCall::SpawnAgent { .. }
        )
    }

    /// Returns true if this ends the agent loop.
    pub fn is_terminal(&self) -> bool {
        matches!(self, ToolCall::TaskComplete { .. })
    }
}

// ── ToolResult ────────────────────────────────────────────────────────────────

/// The outcome of executing a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub output: String,
    pub success: bool,
    pub truncated: bool,
}

impl ToolResult {
    pub fn ok(tool_name: impl Into<String>, output: impl Into<String>) -> Self {
        let output = output.into();
        let truncated = output.len() > MAX_TOOL_OUTPUT;
        let output = if truncated {
            format!("{}\n\n[… output truncated at {} chars …]", &output[..MAX_TOOL_OUTPUT], MAX_TOOL_OUTPUT)
        } else {
            output
        };
        Self { tool_name: tool_name.into(), output, success: true, truncated }
    }

    pub fn err(tool_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            output: format!("ERROR: {}", error.into()),
            success: false,
            truncated: false,
        }
    }
}

/// Maximum characters returned to the LLM from a single tool call.
const MAX_TOOL_OUTPUT: usize = 8_000;

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Parse all `<tool_call>` blocks from a model response.
///
/// Returns an empty vec if the response contains no tool calls (i.e. it is the
/// final answer).
pub fn parse_tool_calls(text: &str) -> Vec<ToolCall> {
    // Match <tool_call name="...">...</tool_call> — possibly multi-line
    let outer_re = Regex::new(r#"(?s)<tool_call\s+name="([^"]+)">(.*?)</tool_call>"#)
        .expect("hardcoded regex is valid");

    let mut calls = Vec::new();

    for cap in outer_re.captures_iter(text) {
        let tool_name = cap[1].trim();
        let body = &cap[2];

        if let Some(call) = parse_single_tool(tool_name, body) {
            calls.push(call);
        }
    }

    calls
}

fn parse_single_tool(name: &str, body: &str) -> Option<ToolCall> {
    match name {
        "read_file" => {
            let path = extract_tag(body, "path")?;
            Some(ToolCall::ReadFile { path })
        }
        "write_file" => {
            let path = extract_tag(body, "path")?;
            let content = extract_tag(body, "content")?;
            Some(ToolCall::WriteFile { path, content })
        }
        "apply_patch" => {
            let path = extract_tag(body, "path")?;
            let patch = extract_tag(body, "patch")?;
            Some(ToolCall::ApplyPatch { path, patch })
        }
        "bash" => {
            let command = extract_tag(body, "command")?;
            Some(ToolCall::Bash { command })
        }
        "search_files" => {
            let query = extract_tag(body, "query")?;
            let glob = extract_tag(body, "glob");
            Some(ToolCall::SearchFiles { query, glob })
        }
        "list_directory" => {
            let path = extract_tag(body, "path").unwrap_or_else(|| ".".to_string());
            Some(ToolCall::ListDirectory { path })
        }
        "web_search" => {
            let query = extract_tag(body, "query")?;
            let num_results = extract_tag(body, "num_results")
                .and_then(|s| s.parse().ok())
                .unwrap_or(5);
            Some(ToolCall::WebSearch { query, num_results })
        }
        "fetch_url" => {
            let url = extract_tag(body, "url")?;
            Some(ToolCall::FetchUrl { url })
        }
        "task_complete" => {
            let summary = extract_tag(body, "summary").unwrap_or_default();
            Some(ToolCall::TaskComplete { summary })
        }
        "spawn_agent" => {
            let task = extract_tag(body, "task")?;
            let max_steps = extract_tag(body, "max_steps")
                .and_then(|s| s.parse().ok());
            Some(ToolCall::SpawnAgent { task, max_steps })
        }
        _ => None,
    }
}

/// Extract content from `<tag>...</tag>` in a body string.
fn extract_tag(body: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"(?s)<{tag}>(.*?)</{tag}>", tag = regex::escape(tag));
    let re = Regex::new(&pattern).ok()?;
    re.captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

// ── Formatting ────────────────────────────────────────────────────────────────

/// Format a tool result to inject back into the conversation as a system/user message.
pub fn format_tool_result(call: &ToolCall, result: &ToolResult) -> String {
    let status = if result.success { "✅" } else { "❌" };
    let truncation_note = if result.truncated {
        "\n[Output was truncated — use more specific search terms or read specific lines]"
    } else {
        ""
    };

    format!(
        "{status} Tool `{}` result:\n```\n{}{}\n```",
        call.name(),
        result.output,
        truncation_note
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_read_file() {
        let text = r#"I'll read the file first.
<tool_call name="read_file">
<path>/src/main.rs</path>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::ReadFile { path } if path == "/src/main.rs"));
    }

    #[test]
    fn test_parse_write_file() {
        let text = r#"<tool_call name="write_file">
<path>hello.rs</path>
<content>
fn main() {
    println!("Hello");
}
</content>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::WriteFile { path, .. } if path == "hello.rs"));
    }

    #[test]
    fn test_parse_bash() {
        let text = r#"<tool_call name="bash">
<command>cargo build 2>&1</command>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::Bash { command } if command.contains("cargo build")));
    }

    #[test]
    fn test_parse_task_complete() {
        let text = r#"<tool_call name="task_complete">
<summary>Done! Created the file.</summary>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::TaskComplete { summary } if summary.contains("Done")));
    }

    #[test]
    fn test_no_tool_calls() {
        let text = "Here is my answer: 42. No tool calls needed.";
        let calls = parse_tool_calls(text);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_tool_result_truncation() {
        let long_output = "x".repeat(MAX_TOOL_OUTPUT + 100);
        let result = ToolResult::ok("read_file", long_output);
        assert!(result.truncated);
        assert!(result.output.len() <= MAX_TOOL_OUTPUT + 200);
    }

    #[test]
    fn test_parse_search_files_with_glob() {
        let text = r#"<tool_call name="search_files">
<query>fn main</query>
<glob>*.rs</glob>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::SearchFiles { query, glob: Some(g) }
            if query == "fn main" && g == "*.rs"));
    }
}
