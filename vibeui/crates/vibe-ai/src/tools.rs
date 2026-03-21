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
The child can spawn its own sub-agents up to `max_depth` levels deep (default: 3, hard max: 5).
```
<tool_call name="spawn_agent">
<task>Write unit tests for src/utils.rs and verify they pass with cargo test.</task>
<max_steps>10</max_steps>
<max_depth>3</max_depth>
</tool_call>
```

### think
Use this tool to reason through complex problems step by step before acting.
Think is free — it does NOT count as a tool execution step. Use it to:
- Break down ambiguous requirements before writing code
- Plan multi-file changes before making them
- Analyze error messages and decide the best fix
- Consider edge cases and potential regressions
```
<tool_call name="think">
<thought>The user wants to add auth. Let me think about what files need to change:
1. Need a middleware for JWT verification
2. Need to update the router to use the middleware
3. Need to add the jsonwebtoken dependency
Let me read the existing router first.</thought>
</tool_call>
```

## Developer Workflow Best Practices

When starting work on a task:
1. **Understand first**: Read relevant files before writing. Use `search_files` to find code patterns.
2. **Think before acting**: Use the `think` tool to plan multi-step changes.
3. **Verify after writing**: Run the project's build/test commands to catch errors early.
4. **Read errors carefully**: When a command fails, read the full error output before retrying.
5. **Prefer apply_patch over write_file**: For modifications to existing files, use `apply_patch` to change only what's needed instead of rewriting the whole file. This is safer and preserves surrounding code.
6. **One concern per step**: Make focused changes. Don't mix unrelated modifications.

When working on a **new (greenfield) project**:
- Start by scaffolding the project structure (package manifest, entry point, config)
- Set up the build/test pipeline immediately
- Add a README.md with setup instructions

When working on an **existing (brownfield) project**:
- Read the README and key config files to understand conventions
- Follow existing code patterns and style
- Run tests after every change to ensure nothing breaks
- Check git status to understand what has changed recently

## Deployment

When the user asks to deploy, ship, publish, or productionize their project, use the `bash` tool.
First check the CLI is installed (`command -v TOOL`), then detect the project type, build if needed, and deploy.

| Platform | CLI | Command |
|----------|-----|---------|
| Vercel | vercel | `vercel deploy --yes` |
| Netlify | netlify | `netlify deploy --prod --dir=dist` |
| Railway | railway | `railway up` |
| AWS App Runner | aws | `copilot deploy` or `aws apprunner create-service` |
| AWS S3 (static) | aws | `npm run build && aws s3 sync dist/ s3://BUCKET --delete` |
| AWS Lambda | serverless | `serverless deploy` |
| AWS ECS/Fargate | aws | docker build → ECR push → `aws ecs update-service --force-new-deployment` |
| Azure App Service | az | `az webapp up --name APP_NAME` |
| Azure Container Apps | az | `az containerapp up --name APP_NAME --source .` |
| Azure Static Web Apps | swa | `swa deploy --output-location dist` |
| GCP Cloud Run | gcloud | `gcloud run deploy --source . --allow-unauthenticated` |
| Firebase | firebase | `firebase deploy --only hosting` |
| DigitalOcean | doctl | `doctl apps create --spec .do/app.yaml` |
| Kubernetes | kubectl | `kubectl apply -f k8s/` |
| Helm | helm | `helm upgrade --install RELEASE .` |
| Oracle Cloud | oci | `fn deploy --app APP` or docker + Container Instance |
| IBM Cloud | ibmcloud | `ibmcloud ce app create --build-source .` |

Auto-detect hints: serverless.yml → Lambda, Dockerfile → container platforms, Chart.yaml → Helm, k8s/ → kubectl, static site → S3/Netlify/Vercel.

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
        /// Maximum recursion depth for sub-agents spawned by this child (default: 3, hard max: 5).
        max_depth: Option<u32>,
    },
    /// Internal reasoning step — lets the agent think through complex problems
    /// without executing any side effects. Does not count toward max_steps.
    Think {
        thought: String,
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
            ToolCall::Think { .. } => "think",
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
                    let end = command.char_indices().nth(60).map(|(i,_)| i).unwrap_or(command.len());
                    format!("{}…", &command[..end])
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
                    let end = summary.char_indices().nth(60).map(|(i,_)| i).unwrap_or(summary.len());
                    format!("{}…", &summary[..end])
                } else {
                    summary.clone()
                };
                format!("task_complete: {}", short)
            }
            ToolCall::SpawnAgent { task, max_steps, max_depth } => {
                let short = if task.len() > 60 { let end = task.char_indices().nth(60).map(|(i,_)| i).unwrap_or(task.len()); format!("{}…", &task[..end]) } else { task.clone() };
                format!("spawn_agent(task={:?}, max_steps={}, max_depth={})", short, max_steps.unwrap_or(10), max_depth.unwrap_or(3))
            }
            ToolCall::Think { thought } => {
                let short = if thought.len() > 80 { let end = thought.char_indices().nth(80).map(|(i,_)| i).unwrap_or(thought.len()); format!("{}…", &thought[..end]) } else { thought.clone() };
                format!("think({})", short)
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

    /// Returns true if this is a no-op reasoning step (think tool).
    pub fn is_think(&self) -> bool {
        matches!(self, ToolCall::Think { .. })
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
            let max_depth = extract_tag(body, "max_depth")
                .and_then(|s| s.parse().ok());
            Some(ToolCall::SpawnAgent { task, max_steps, max_depth })
        }
        "think" => {
            let thought = extract_tag(body, "thought").unwrap_or_default();
            Some(ToolCall::Think { thought })
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

    // ── ToolCall::name() ─────────────────────────────────────────────────

    #[test]
    fn tool_call_names() {
        assert_eq!(ToolCall::ReadFile { path: "a".into() }.name(), "read_file");
        assert_eq!(ToolCall::WriteFile { path: "a".into(), content: "b".into() }.name(), "write_file");
        assert_eq!(ToolCall::ApplyPatch { path: "a".into(), patch: "b".into() }.name(), "apply_patch");
        assert_eq!(ToolCall::Bash { command: "ls".into() }.name(), "bash");
        assert_eq!(ToolCall::SearchFiles { query: "q".into(), glob: None }.name(), "search_files");
        assert_eq!(ToolCall::ListDirectory { path: ".".into() }.name(), "list_directory");
        assert_eq!(ToolCall::WebSearch { query: "q".into(), num_results: 5 }.name(), "web_search");
        assert_eq!(ToolCall::FetchUrl { url: "u".into() }.name(), "fetch_url");
        assert_eq!(ToolCall::TaskComplete { summary: "s".into() }.name(), "task_complete");
        assert_eq!(ToolCall::SpawnAgent { task: "t".into(), max_steps: None, max_depth: None }.name(), "spawn_agent");
    }

    // ── ToolCall::is_destructive() ───────────────────────────────────────

    #[test]
    fn is_destructive_true_for_bash() {
        assert!(ToolCall::Bash { command: "rm -rf /".into() }.is_destructive());
    }

    #[test]
    fn is_destructive_true_for_write() {
        assert!(ToolCall::WriteFile { path: "a".into(), content: "b".into() }.is_destructive());
    }

    #[test]
    fn is_destructive_true_for_patch() {
        assert!(ToolCall::ApplyPatch { path: "a".into(), patch: "b".into() }.is_destructive());
    }

    #[test]
    fn is_destructive_true_for_spawn() {
        assert!(ToolCall::SpawnAgent { task: "t".into(), max_steps: None, max_depth: None }.is_destructive());
    }

    #[test]
    fn is_destructive_false_for_read() {
        assert!(!ToolCall::ReadFile { path: "a".into() }.is_destructive());
        assert!(!ToolCall::SearchFiles { query: "q".into(), glob: None }.is_destructive());
        assert!(!ToolCall::ListDirectory { path: ".".into() }.is_destructive());
        assert!(!ToolCall::WebSearch { query: "q".into(), num_results: 5 }.is_destructive());
        assert!(!ToolCall::FetchUrl { url: "u".into() }.is_destructive());
        assert!(!ToolCall::TaskComplete { summary: "done".into() }.is_destructive());
    }

    // ── ToolCall::is_terminal() ──────────────────────────────────────────

    #[test]
    fn is_terminal_only_for_task_complete() {
        assert!(ToolCall::TaskComplete { summary: "done".into() }.is_terminal());
        assert!(!ToolCall::ReadFile { path: "a".into() }.is_terminal());
        assert!(!ToolCall::Bash { command: "ls".into() }.is_terminal());
    }

    // ── ToolCall::summary() ──────────────────────────────────────────────

    #[test]
    fn summary_read_file() {
        let s = ToolCall::ReadFile { path: "/src/main.rs".into() }.summary();
        assert_eq!(s, "read_file(/src/main.rs)");
    }

    #[test]
    fn summary_write_file_counts_lines() {
        let s = ToolCall::WriteFile { path: "a.rs".into(), content: "line1\nline2\nline3\n".into() }.summary();
        assert!(s.contains("3 lines"), "got: {}", s);
    }

    #[test]
    fn summary_apply_patch_counts_hunks() {
        let patch = "@@ -1,3 +1,4 @@\n foo\n+bar\n@@ -10,2 +11,3 @@\n baz\n+qux\n";
        let s = ToolCall::ApplyPatch { path: "a.rs".into(), patch: patch.into() }.summary();
        assert!(s.contains("2 hunks"), "got: {}", s);
    }

    #[test]
    fn summary_bash_truncates_long_command() {
        let long_cmd = "a".repeat(100);
        let s = ToolCall::Bash { command: long_cmd }.summary();
        assert!(s.contains("…"), "long command should be truncated");
        assert!(s.len() < 100);
    }

    #[test]
    fn summary_search_with_glob() {
        let s = ToolCall::SearchFiles { query: "foo".into(), glob: Some("*.rs".into()) }.summary();
        assert!(s.contains("*.rs"), "got: {}", s);
    }

    #[test]
    fn summary_search_without_glob() {
        let s = ToolCall::SearchFiles { query: "bar".into(), glob: None }.summary();
        assert!(s.contains("bar") && !s.contains("in"), "got: {}", s);
    }

    #[test]
    fn summary_spawn_agent() {
        let s = ToolCall::SpawnAgent { task: "do stuff".into(), max_steps: Some(5), max_depth: Some(2) }.summary();
        assert!(s.contains("max_steps=5"), "got: {}", s);
        assert!(s.contains("max_depth=2"), "got: {}", s);
    }

    #[test]
    fn summary_spawn_agent_defaults() {
        let s = ToolCall::SpawnAgent { task: "x".into(), max_steps: None, max_depth: None }.summary();
        assert!(s.contains("max_steps=10"), "default should be 10, got: {}", s);
        assert!(s.contains("max_depth=3"), "default should be 3, got: {}", s);
    }

    // ── ToolResult ───────────────────────────────────────────────────────

    #[test]
    fn tool_result_ok_short_output() {
        let r = ToolResult::ok("read_file", "hello");
        assert!(r.success);
        assert!(!r.truncated);
        assert_eq!(r.output, "hello");
        assert_eq!(r.tool_name, "read_file");
    }

    #[test]
    fn tool_result_ok_truncates_long_output() {
        let long = "x".repeat(MAX_TOOL_OUTPUT + 500);
        let r = ToolResult::ok("bash", long);
        assert!(r.truncated);
        assert!(r.success);
        assert!(r.output.contains("truncated"));
    }

    #[test]
    fn tool_result_err() {
        let r = ToolResult::err("bash", "command not found");
        assert!(!r.success);
        assert!(!r.truncated);
        assert!(r.output.starts_with("ERROR:"));
        assert!(r.output.contains("command not found"));
    }

    // ── format_tool_result ───────────────────────────────────────────────

    #[test]
    fn format_tool_result_success() {
        let call = ToolCall::ReadFile { path: "a.rs".into() };
        let result = ToolResult { tool_name: "read_file".into(), output: "fn main() {}".into(), success: true, truncated: false };
        let formatted = format_tool_result(&call, &result);
        assert!(formatted.starts_with("✅"));
        assert!(formatted.contains("read_file"));
        assert!(formatted.contains("fn main()"));
    }

    #[test]
    fn format_tool_result_error() {
        let call = ToolCall::Bash { command: "bad".into() };
        let result = ToolResult::err("bash", "not found");
        let formatted = format_tool_result(&call, &result);
        assert!(formatted.starts_with("❌"));
    }

    #[test]
    fn format_tool_result_truncated_note() {
        let call = ToolCall::Bash { command: "cat big".into() };
        let result = ToolResult { tool_name: "bash".into(), output: "data".into(), success: true, truncated: true };
        let formatted = format_tool_result(&call, &result);
        assert!(formatted.contains("truncated"));
    }

    // ── parse edge cases ─────────────────────────────────────────────────

    #[test]
    fn parse_list_directory_default_path() {
        let text = r#"<tool_call name="list_directory">
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::ListDirectory { path } if path == "."));
    }

    #[test]
    fn parse_web_search() {
        let text = r#"<tool_call name="web_search">
<query>rust async</query>
<num_results>3</num_results>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::WebSearch { query, num_results: 3 } if query == "rust async"));
    }

    #[test]
    fn parse_web_search_default_num_results() {
        let text = r#"<tool_call name="web_search">
<query>hello</query>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert!(matches!(&calls[0], ToolCall::WebSearch { num_results: 5, .. }));
    }

    #[test]
    fn parse_fetch_url() {
        let text = r#"<tool_call name="fetch_url">
<url>https://example.com</url>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert!(matches!(&calls[0], ToolCall::FetchUrl { url } if url == "https://example.com"));
    }

    #[test]
    fn parse_spawn_agent() {
        let text = r#"<tool_call name="spawn_agent">
<task>Write tests</task>
<max_steps>5</max_steps>
<max_depth>2</max_depth>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert!(matches!(&calls[0], ToolCall::SpawnAgent { task, max_steps: Some(5), max_depth: Some(2) } if task == "Write tests"));
    }

    #[test]
    fn parse_unknown_tool_ignored() {
        let text = r#"<tool_call name="delete_universe">
<target>everything</target>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_multiple_tool_calls() {
        let text = r#"
<tool_call name="read_file">
<path>a.rs</path>
</tool_call>
Some text in between
<tool_call name="bash">
<command>ls</command>
</tool_call>
"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name(), "read_file");
        assert_eq!(calls[1].name(), "bash");
    }
}
