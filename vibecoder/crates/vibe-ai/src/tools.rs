//! Universal prompt-based tool framework.
//!
//! Works with every LLM provider (Ollama, Claude, OpenAI, Gemini, Grok) by injecting
//! tool definitions into the system prompt and parsing `<tool_call>` XML blocks from
//! model output — no native function-calling API required.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ── System Prompt ─────────────────────────────────────────────────────────────

/// System prompt fragment that teaches the model how to call tools.
/// Prepended to every agent conversation.
pub const TOOL_SYSTEM_PROMPT: &str = r#"
You are Vibe Agent, an autonomous coding agent running in the user's terminal.
Your name is "Vibe Agent" — always refer to yourself as Vibe Agent, never as VibeCLI or any other name.

## Tool Use

**CRITICAL**: Every response MUST be a tool call. Never write planning text, explanations, or prose before acting. Call a tool immediately — no exceptions.

To use a tool, output ONLY a single `<tool_call>` block with NO other text in the response:

```
<tool_call name="TOOL_NAME">
<param_name>param value</param_name>
</tool_call>
```

After each tool result is shown to you, call the next tool. Never call more than
one tool per response. When the task is fully complete, call `task_complete`.

### Escaping parameter values

Parameter values are XML text nodes. Inside them, escape `&` as `&amp;`, `<` as
`&lt;` and `>` as `&gt;`. Quotes, backslashes and newlines are literal — do not
escape them. To write a literal entity into a file (an HTML `&amp;`, say), emit
`&amp;amp;`.

**Wrong** (DO NOT do this):
```
I'll start by exploring the repository...
<tool_call name="list_directory">...</tool_call>
```

**Correct** (do this immediately):
```
<tool_call name="list_directory">
<path>.</path>
</tool_call>
```

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

**Call it as soon as the work is done.** Re-reading files you have already read,
re-writing a file with content it already has, or restating the plan is not
progress — it burns the user's time budget and can get the run killed with the
work finished but unreported. If you believe you are done, say so and stop.

**Your summary must be true.** Only claim a test passes if you ran it and saw it
pass. Only claim you changed something if you actually wrote it. A confident
false summary is worse than an honest report of partial work, because the user
acts on it.

**Never weaken a security control to make a check pass.** If a test asserts that
authentication, authorization, validation or a permission check should let
something through that it currently blocks, the test is the thing that is wrong.
Leave the control in place, call `task_complete`, and state plainly in your
summary that the test asserts unsafe behaviour and was not satisfied. Removing an
auth check to turn a suite green is never the correct fix.

### spawn_agent
Delegate an independent sub-task to a child agent. The child runs with the same tools and
workspace. Use this to parallelize work or isolate complex sub-problems.
The child can spawn its own sub-agents up to `max_depth` levels deep (default: 3, hard max: 5).

**IMPORTANT constraints:**
- Only spawn an agent for a task that is **explicitly part of the user's current request**.
- Do NOT invent side-tasks (e.g., writing tests, adding docs) unless the user asked for them.
- Do NOT copy or adapt the example task below — it is illustrative only.
```
<tool_call name="spawn_agent">
<task>Implement the authentication module described in the requirements.</task>
<max_steps>15</max_steps>
<max_depth>2</max_depth>
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

### plan_task
Break the current task into numbered steps. Call this FIRST for complex multi-file tasks. It does not execute anything — just displays your plan.
```
<tool_call name="plan_task">
<steps>
1. Read current schema in src/db.rs
2. Add migration file
3. Update model struct
4. Run cargo test
</steps>
</tool_call>
```

### diffstat
Show what changed in a file compared to git HEAD. Use before calling task_complete to summarize changes.
```
<tool_call name="diffstat">
<path>src/main.rs</path>
</tool_call>
```

### record_memory
Save a key insight to persistent memory for future agent sessions. Use to record discovered file locations, error patterns, and important context.
```
<tool_call name="record_memory">
<key>database_url_location</key>
<value>Found in src/config.rs line 42</value>
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

## CRITICAL: Break Large Tasks Into Small Steps

**NEVER generate an entire project or multiple files in a single response.**
Large code generation WILL fail with response size limits and wastes tokens on retries.

Instead, follow this incremental approach:
1. Use `think` to plan: list every file that needs to be created/modified
2. Write ONE file per tool call — keep each file focused and complete
3. For large files (>200 lines), write the skeleton first, then fill in sections
4. After every 2-3 files, run `build` or type-check to catch errors early
5. Use `spawn_agent` only when the user explicitly asked for a parallel sub-task (e.g., "also write tests", "generate docs too")

**File creation order:**
- Config files first (package.json, Cargo.toml, tsconfig, etc.)
- Core types/interfaces
- Implementation modules (one at a time)
- Tests (only if the user requested them)
- Documentation last

When working on a **new (greenfield) project**:
- Start by scaffolding the project structure (package manifest, entry point, config)
- Set up the build/test pipeline immediately
- Create files ONE AT A TIME — never batch multiple files into one response
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

## Doing Tasks

You are highly capable and often allow users to complete ambitious tasks that would otherwise be too complex or take too long. Defer to user judgement about whether a task is too large.

The user will primarily request software engineering tasks: solving bugs, adding features, refactoring code, explaining code, and more. When given an unclear or generic instruction, consider it in the context of software engineering and the current workspace.

**Read before modifying.** Do not propose changes to code you haven't read. If a user asks about or wants you to modify a file, read it first. Understand existing code before suggesting modifications.

**Minimize file creation.** Do not create files unless absolutely necessary. Prefer editing existing files over creating new ones — this prevents file bloat and builds on existing work.

**No unnecessary additions.** Don't add features, refactor code, or make "improvements" beyond what was asked. A bug fix doesn't need surrounding code cleaned up. A simple feature doesn't need extra configurability. Don't add docstrings, comments, or type annotations to code you didn't change. Only add comments where the logic isn't self-evident.

**No premature abstractions.** Don't create helpers, utilities, or abstractions for one-time operations. Don't design for hypothetical future requirements. Three similar lines of code is better than a premature abstraction. The right amount of complexity is what the task actually requires.

**No unnecessary error handling.** Don't add error handling, fallbacks, or validation for scenarios that can't happen. Trust internal code and framework guarantees. Only validate at system boundaries (user input, external APIs).

**No compatibility hacks.** Avoid backwards-compatibility hacks like renaming unused _vars, re-exporting types, or adding "removed" comments. If something is unused, delete it completely.

**Security first.** Be careful not to introduce security vulnerabilities: command injection, XSS, SQL injection, and other OWASP top 10. If you notice insecure code, fix it immediately. Prioritize writing safe, secure, and correct code.

## Executing Actions with Care

Carefully consider the reversibility and blast radius of actions. You can freely take local, reversible actions like editing files or running tests. But for actions that are hard to reverse, affect shared systems, or could be destructive, check with the user first.

Risky actions that warrant confirmation:
- Destructive: deleting files/branches, dropping tables, rm -rf, overwriting uncommitted changes
- Hard-to-reverse: force-pushing, git reset --hard, amending published commits, modifying CI/CD
- Visible to others: pushing code, creating/commenting on PRs/issues, sending messages

When you encounter an obstacle, do not use destructive actions as a shortcut. Identify root causes and fix underlying issues rather than bypassing safety checks (e.g. --no-verify). If you discover unexpected state, investigate before deleting or overwriting — it may be the user's in-progress work.

## Output Efficiency

Go straight to the point. Try the simplest approach first. Be extra concise.

Keep text output brief and direct. Lead with the answer or action, not the reasoning. Skip filler words, preamble, and unnecessary transitions. Do not restate what the user said.

Focus text output on:
- Decisions that need user input
- High-level status updates at natural milestones
- Errors or blockers that change the plan

If you can say it in one sentence, don't use three.

## Writing Sub-Agent Prompts

When using `spawn_agent`, brief the agent like a smart colleague who just walked into the room — it hasn't seen this conversation, doesn't know what you've tried.
- Explain what you're trying to accomplish and why.
- Describe what you've already learned or ruled out.
- Give enough context that the agent can make judgment calls.
- Terse command-style prompts produce shallow, generic work.

**Never delegate understanding.** Don't write "based on your findings, fix the bug." Write prompts that prove you understood: include file paths, line numbers, what specifically to change.

## Safety

Assist with authorized security testing, defensive security, CTF challenges, and educational contexts. Refuse requests for destructive techniques, DoS attacks, mass targeting, supply chain compromise, or detection evasion for malicious purposes. Dual-use security tools (C2 frameworks, credential testing, exploit development) require clear authorization context: pentesting engagements, CTF competitions, security research, or defensive use cases.

Do not help create malware, ransomware, keyloggers, or tools designed to harm users. Do not assist with social engineering attacks targeting real individuals. Do not generate content that could be used to bypass authentication systems without authorization.
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
    /// Break the task into numbered steps for display. Pure display — no side effects.
    PlanTask {
        steps: String,
    },
    /// Show git diff --stat for a file compared to HEAD.
    Diffstat {
        path: String,
    },
    /// Save a key/value insight to .vibe/memory.md for future sessions.
    RecordMemory {
        key: String,
        value: String,
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
            ToolCall::PlanTask { .. } => "plan_task",
            ToolCall::Diffstat { .. } => "diffstat",
            ToolCall::RecordMemory { .. } => "record_memory",
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
                    let end = command
                        .char_indices()
                        .nth(60)
                        .map(|(i, _)| i)
                        .unwrap_or(command.len());
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
                    let end = summary
                        .char_indices()
                        .nth(60)
                        .map(|(i, _)| i)
                        .unwrap_or(summary.len());
                    format!("{}…", &summary[..end])
                } else {
                    summary.clone()
                };
                format!("task_complete: {}", short)
            }
            ToolCall::SpawnAgent {
                task,
                max_steps,
                max_depth,
            } => {
                let short = if task.len() > 60 {
                    let end = task
                        .char_indices()
                        .nth(60)
                        .map(|(i, _)| i)
                        .unwrap_or(task.len());
                    format!("{}…", &task[..end])
                } else {
                    task.clone()
                };
                format!(
                    "spawn_agent(task={:?}, max_steps={}, max_depth={})",
                    short,
                    max_steps.unwrap_or(10),
                    max_depth.unwrap_or(3)
                )
            }
            ToolCall::Think { thought } => {
                let short = if thought.len() > 80 {
                    let end = thought
                        .char_indices()
                        .nth(80)
                        .map(|(i, _)| i)
                        .unwrap_or(thought.len());
                    format!("{}…", &thought[..end])
                } else {
                    thought.clone()
                };
                format!("think({})", short)
            }
            ToolCall::PlanTask { steps } => {
                let lines = steps.lines().count();
                format!("plan_task({} steps)", lines)
            }
            ToolCall::Diffstat { path } => format!("diffstat({})", path),
            ToolCall::RecordMemory { key, .. } => format!("record_memory(key={})", key),
        }
    }

    /// Returns true if this is a destructive / risky operation.
    pub fn is_destructive(&self) -> bool {
        match self {
            // Inspection commands are not destructive; labelling `find … | sort`
            // as one trains users to click through the warning that matters.
            ToolCall::Bash { command } => !bash_is_read_only(command),
            ToolCall::WriteFile { .. }
            | ToolCall::ApplyPatch { .. }
            | ToolCall::SpawnAgent { .. }
            | ToolCall::RecordMemory { .. } => true,
            _ => false,
        }
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
            format!(
                "{}\n\n[… output truncated at {} chars …]",
                &output[..MAX_TOOL_OUTPUT],
                MAX_TOOL_OUTPUT
            )
        } else {
            output
        };
        Self {
            tool_name: tool_name.into(),
            output,
            success: true,
            truncated,
        }
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

/// Shell commands that only inspect state. Anything not on this list is
/// treated as potentially destructive.
const READ_ONLY_COMMANDS: &[&str] = &[
    "ls", "find", "grep", "rg", "cat", "head", "tail", "wc", "sort", "uniq", "cut", "tr", "pwd",
    "tree", "stat", "file", "du", "df", "basename", "dirname", "echo", "printf", "which", "type",
    "date", "whoami", "hostname", "uname", "column", "nl", "jq", "yq", "diff", "cmp", "md5",
    "true",
];

/// `git` subcommands that only read the repository.
const READ_ONLY_GIT_SUBCOMMANDS: &[&str] = &[
    "status",
    "log",
    "diff",
    "show",
    "ls-files",
    "rev-parse",
    "blame",
    "describe",
    "shortlog",
    "branch",
    "remote",
    "config",
];

/// Shell syntax that can turn an inspection command into a mutating one.
const UNSAFE_SHELL_TOKENS: &[&str] = &[
    ">", "<", ";", "&", "`", "$(", "${", "\n", "--delete", "-delete", "-exec", "-execdir", "-ok",
    "-fprint", "-fls",
];

/// True when `command` only inspects state — every pipeline segment starts with
/// a known read-only command and the line carries no redirection, chaining, or
/// substitution that could smuggle in a mutation.
///
/// Deliberately conservative: an unrecognised command is "not read-only", so a
/// false negative costs an extra approval prompt while a false positive would
/// hide a real warning.
pub fn bash_is_read_only(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }
    if UNSAFE_SHELL_TOKENS.iter().any(|t| trimmed.contains(t)) {
        return false;
    }
    trimmed.split('|').all(|segment| {
        let mut words = segment.split_whitespace();
        // Skip leading `env`-style prefixes? No — keep it strict.
        let Some(head) = words.next() else {
            return false;
        };
        // Strip a path prefix: /usr/bin/find → find
        let head = head.rsplit('/').next().unwrap_or(head);
        if head == "git" {
            return words
                .find(|w| !w.starts_with('-'))
                .is_some_and(|sub| READ_ONLY_GIT_SUBCOMMANDS.contains(&sub));
        }
        READ_ONLY_COMMANDS.contains(&head)
    })
}

/// Every tool name [`parse_tool_calls`] recognises. Used to tell a model that
/// reached for something else (gpt-oss likes its built-in `container.exec`)
/// what it can actually call.
pub const AVAILABLE_TOOL_NAMES: &[&str] = &[
    "read_file",
    "write_file",
    "apply_patch",
    "bash",
    "search_files",
    "list_directory",
    "web_search",
    "fetch_url",
    "task_complete",
    "spawn_agent",
    "think",
    "plan_task",
    "diffstat",
    "record_memory",
];

/// The same tools as [`TOOL_SYSTEM_PROMPT`], in the machine-readable shape that
/// Ollama and every OpenAI-compatible endpoint expect on a request's `tools`
/// field.
///
/// Describing tools *only* in the system prompt works for models that follow
/// prose instructions, but a model trained for native tool calling has nothing
/// to call: it narrates its intent ("Let me check the workspace…") and emits an
/// empty turn, which reads as the agent hanging. Declaring them here is the
/// missing half of a round trip whose other end already exists — providers
/// transcribe native calls back to `<tool_call>` markup via
/// [`render_tool_call`].
///
/// Parameter names are load-bearing: `render_tool_call` turns each JSON key
/// into an XML tag, so they must match what `parse_single_tool` extracts.
/// `tool_definitions_match_parser` pins that.
/// Substring unique to [`TOOL_SYSTEM_PROMPT`], used to recognise a conversation
/// the agent loop built.
pub const TOOL_PROMPT_MARKER: &str = "## Available Tools";

/// True when this conversation carries the agent's tool system prompt, and so
/// expects tool calls.
///
/// Providers advertise [`tool_definitions`] only for these. A plain chat panel
/// never asked for tools, and handing them to the model there would produce
/// `<tool_call>` markup that the panel renders as literal text — the same class
/// of leak as raw `<thinking>` tags.
pub fn expects_tools<'a>(message_contents: impl IntoIterator<Item = &'a str>) -> bool {
    message_contents
        .into_iter()
        .any(|c| c.contains(TOOL_PROMPT_MARKER))
}

pub fn tool_definitions() -> Vec<serde_json::Value> {
    /// One tool, as an OpenAI-shaped function schema.
    fn tool(
        name: &str,
        description: &str,
        params: &[(&str, &str, &str)], // (name, json type, description)
        required: &[&str],
    ) -> serde_json::Value {
        let properties: serde_json::Map<String, serde_json::Value> = params
            .iter()
            .map(|(p, ty, desc)| {
                (
                    (*p).to_string(),
                    serde_json::json!({ "type": ty, "description": desc }),
                )
            })
            .collect();
        serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                },
            }
        })
    }

    vec![
        tool(
            "read_file",
            "Read the contents of a file at the given path.",
            &[("path", "string", "Path to the file to read.")],
            &["path"],
        ),
        tool(
            "write_file",
            "Write (create or overwrite) a file. Content must be the complete file.",
            &[
                ("path", "string", "Path to the file to write."),
                ("content", "string", "Full contents of the file."),
            ],
            &["path", "content"],
        ),
        tool(
            "apply_patch",
            "Apply a unified diff to an existing file.",
            &[
                ("path", "string", "Path to the file to patch."),
                ("patch", "string", "Unified diff to apply."),
            ],
            &["path", "patch"],
        ),
        tool(
            "bash",
            "Execute a shell command and return stdout + stderr.",
            &[("command", "string", "Shell command to run.")],
            &["command"],
        ),
        tool(
            "search_files",
            "Search for files matching a pattern or containing specific text.",
            &[
                ("query", "string", "Search term or regular expression."),
                (
                    "glob",
                    "string",
                    "Optional glob to restrict the search, e.g. *.rs",
                ),
            ],
            &["query"],
        ),
        tool(
            "list_directory",
            "List files and directories at the given path.",
            &[(
                "path",
                "string",
                "Directory to list. Defaults to the workspace root.",
            )],
            &[],
        ),
        tool(
            "web_search",
            "Search the web for current information.",
            &[
                ("query", "string", "What to search for."),
                (
                    "num_results",
                    "integer",
                    "How many results to return. Defaults to 5.",
                ),
            ],
            &["query"],
        ),
        tool(
            "fetch_url",
            "Fetch a web page and extract its text content.",
            &[("url", "string", "Absolute URL to fetch.")],
            &["url"],
        ),
        tool(
            "task_complete",
            "Call when the task is fully done, with the final summary for the user.",
            &[(
                "summary",
                "string",
                "Final summary of what was accomplished.",
            )],
            &["summary"],
        ),
        tool(
            "spawn_agent",
            "Delegate a self-contained sub-task to a nested agent.",
            &[
                ("task", "string", "The sub-task to delegate."),
                (
                    "max_steps",
                    "integer",
                    "Optional step budget for the sub-agent.",
                ),
                ("max_depth", "integer", "Optional nesting depth limit."),
            ],
            &["task"],
        ),
        tool(
            "think",
            "Record a private reasoning step without acting.",
            &[("thought", "string", "The reasoning to record.")],
            &["thought"],
        ),
        tool(
            "plan_task",
            "Record a step-by-step plan before executing it.",
            &[("steps", "string", "The planned steps.")],
            &["steps"],
        ),
        tool(
            "diffstat",
            "Summarise pending changes for a path.",
            &[("path", "string", "Path to summarise.")],
            &["path"],
        ),
        tool(
            "record_memory",
            "Persist a durable fact for later turns.",
            &[
                ("key", "string", "Short identifier for the memory."),
                ("value", "string", "The fact to remember."),
            ],
            &["key", "value"],
        ),
    ]
}

/// Name of a `<tool_call>` block that [`parse_tool_calls`] could not turn into
/// a call — an unknown tool, or one missing required parameters.
///
/// Returns `None` when the text has no tool-call markup at all (genuine prose)
/// or when every block parsed successfully. Reasoning is ignored, as in
/// [`parse_tool_calls`].
/// A precise, actionable rejection message for a tool call that failed to
/// parse — the text fed back to the model so it can retry.
///
/// The distinction matters. When a model emits `<command>pwd</arg_value>` the
/// tool `bash` is perfectly valid and only the closing tag is wrong; telling it
/// "`bash` is not an available tool" is false, and a model that believes it
/// stops reaching for the tool it actually needed. Naming the real fault —
/// unknown tool vs. malformed parameter tags — is what makes the retry work.
pub fn tool_call_rejection_reason(text: &str) -> Option<String> {
    let name = unparsed_tool_call_name(text)?;

    if !AVAILABLE_TOOL_NAMES.contains(&name.as_str()) {
        return Some(format!(
            "Tool call rejected: `{}` is not an available tool. You have exactly \
             these tools: {}. Retry now with one of them.",
            name,
            AVAILABLE_TOOL_NAMES.join(", "),
        ));
    }

    // Known tool, so the block itself is at fault. Quote the exact tags it
    // needs — the common failure is an opening tag closed by a different name.
    let required: Vec<String> = tool_definitions()
        .into_iter()
        .find(|d| d["function"]["name"] == name.as_str())
        .and_then(|d| d["function"]["parameters"]["required"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(|s| format!("<{s}>…</{s}>")))
        .collect();

    Some(format!(
        "Tool call rejected: the `{}` block was malformed and could not be parsed. \
         Every parameter must be wrapped in a matching pair of tags — the closing \
         tag must repeat the opening tag's name exactly.{} \
         Retry now, emitting the whole block again.",
        name,
        match required.is_empty() {
            true => String::new(),
            false => format!(" `{}` requires: {}.", name, required.join(", ")),
        },
    ))
}

/// The canonical `<tool_call name="…">…</tool_call>` regex, compiled once.
///
/// Two call sites had their own copy of this pattern and each recompiled it per
/// call; one shared static keeps them from drifting as well as from allocating.
fn tool_call_re() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)<tool_call\s+name="([^"]+)">(.*?)</tool_call>"#)
            .expect("hardcoded regex is valid")
    });
    &RE
}

pub fn unparsed_tool_call_name(text: &str) -> Option<String> {
    let visible = strip_thinking(text);
    for cap in tool_call_re().captures_iter(&visible) {
        let name = cap[1].trim();
        if parse_single_tool(name, &cap[2]).is_none() {
            return Some(name.to_string());
        }
    }
    // An opening tag with no closing tag never reaches the loop above.
    if visible.contains("<tool_call") && parse_tool_calls(&visible).is_empty() {
        return Some("(malformed tool_call block)".to_string());
    }
    None
}

/// Render a native (structured) tool call as the `<tool_call name="…">` markup
/// [`parse_tool_calls`] understands.
///
/// Providers whose APIs return function calls as JSON — rather than as the
/// markup this crate's prompt asks for — transcribe them through here so the
/// agent loop sees one canonical form. Non-string argument values keep their
/// JSON representation (`3`, `["a"]`), which is what the per-tool parsers
/// expect for numeric and structured fields.
pub fn render_tool_call(name: &str, args: Option<&serde_json::Value>) -> String {
    let body = args
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(key, value)| {
                    let rendered = match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    // Escaped because the parser decodes (`extract_tag`). A
                    // native tool call carrying `&` or `<` — any Rust or HTML
                    // file body — otherwise fails to survive the transcription
                    // round-trip that providers like Ollama route through.
                    format!("<{k}>{v}</{k}>", k = key, v = escape_xml_text(&rendered))
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    format!("<tool_call name=\"{}\">{}</tool_call>", name, body)
}

/// A reasoning-tag family, compiled once.
///
/// Whatever the tag is called, reasoning arrives in the same three shapes — a
/// closed block, an *orphan* close (the provider swallowed the opening tag
/// into its own reasoning field), and an opening tag the stream never closed —
/// so all four regexes are built from one name alternation.
struct TagStripper {
    block: Regex,
    orphan: Regex,
    unclosed: Regex,
    tags: Regex,
}

impl TagStripper {
    fn new(names: &str) -> Self {
        let open = format!(r"<(?:[A-Za-z][\w.-]*:)?(?:{names})>");
        let close = format!(r"</(?:[A-Za-z][\w.-]*:)?(?:{names})>");
        let rx = |pattern: String| Regex::new(&pattern).expect("hardcoded regex is valid");
        Self {
            block: rx(format!("(?s){open}.*?{close}")),
            orphan: rx(format!("(?s)^.*?{close}")),
            unclosed: rx(format!("(?s){open}.*$")),
            tags: rx(format!(r"</?(?:[A-Za-z][\w.-]*:)?(?:{names})>")),
        }
    }

    /// Drop the reasoning, keep what surrounds it.
    fn strip(&self, text: &str) -> String {
        let stripped = self.block.replace_all(text, "");
        let stripped = self.orphan.replace(&stripped, "").into_owned();
        self.unclosed.replace(&stripped, "").into_owned()
    }

    /// Keep the text, drop only the tags.
    fn unwrap(&self, text: &str) -> String {
        self.tags.replace_all(text, "").trim().to_string()
    }
}

/// Reasoning tag spellings we recognise.
///
/// Models do not agree on this tag. `<think>` (GLM/Qwen/R1), `<thinking>`
/// (Claude-style, and what our own provider layer emits), and **namespaced**
/// forms like minimax-m3's `<mm:think>` all occur. A stripper that misses one
/// spelling leaks raw reasoning into the answer, which is what `</mm:think>`
/// did — it appeared verbatim in the chat window, mid-sentence.
static THINK: LazyLock<TagStripper> = LazyLock::new(|| TagStripper::new(r"think(?:ing)?"));

/// The wider reasoning family, for short one-shot answers.
///
/// Only for paths where the whole reply is a single short artifact (a commit
/// message, a title) and *no* legitimate output contains these tags. The chat
/// and tool-parsing paths must keep to [`THINK`]: a chat turn may quite
/// reasonably contain an `<analysis>` element as content.
static REASONING: LazyLock<TagStripper> = LazyLock::new(|| {
    TagStripper::new(
        r"think(?:ing)?|reason(?:ing)?|reflection|reflect|scratch_?pad|analysis|internal(?:_monologue)?|monologue",
    )
});

/// Wrappers a model puts *around* its answer rather than around its reasoning.
/// The text inside is the answer, so these lose their tags and keep their body.
static ANSWER_TAGS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)</?(?:[A-Za-z][\w.-]*:)?(?:final_?answer|answer|final|output|response|commit_?message|commit_?msg)>",
    )
    .expect("hardcoded regex is valid")
});

/// Remove reasoning *tags* while keeping the text inside them.
///
/// For when a turn is nothing but reasoning and that reasoning is the reply.
/// Some models (minimax-m3, observed putting a 54k-character answer inside a
/// single `<thinking>` block) never emit content outside it, so
/// [`strip_thinking`] correctly returns nothing — and the choice is between
/// showing the user an empty turn or unwrapping what the model actually said.
///
/// Only for display paths. Tool parsing must keep using `strip_thinking`,
/// because reasoning quotes calls the model then rejected.
pub fn unwrap_thinking(text: &str) -> String {
    THINK.unwrap(text)
}

/// Remove `<thinking>` / `<think>` reasoning blocks from a model response.
///
/// Reasoning routinely *quotes* the call the model is considering — including
/// calls it then rejects — so tool parsing must run on the visible turn only.
/// An unclosed block (the stream ended, or was gated, mid-reasoning) discards
/// everything after the opening tag.
pub fn strip_thinking(text: &str) -> String {
    THINK.strip(text)
}

/// Remove a single wrapping ``` fence if the whole response is one.
///
/// Only strips when the response *starts* with a fence, so a reply that
/// legitimately contains a fence (writing a doc comment, say) is untouched.
pub fn strip_code_fence(raw: &str) -> &str {
    let trimmed = raw.trim_matches('\n');
    let Some(rest) = trimmed.strip_prefix("```") else {
        return raw;
    };
    // Drop the info string (```rust) on the opening fence.
    let after_open = match rest.find('\n') {
        Some(nl) => &rest[nl + 1..],
        // A fence with no newline has no body.
        None => return "",
    };
    after_open
        .rfind("```")
        .map_or(after_open, |close| &after_open[..close])
}

/// Reduce a model's reply to the commit message it was asked for.
///
/// A commit message is written to the repository verbatim and read by everyone
/// afterwards, so transport markup that is merely ugly in a chat window is
/// permanent here: `<thinking>` blocks, deliberation about the message, and the
/// fence the model wrapped it in have all been committed verbatim by this
/// generator — see the reported subjects in the tests.
///
/// Returns an empty string when the reply held no message. Callers must treat
/// that as a failure — committing an empty message is worse than not
/// committing.
pub fn sanitize_commit_message(raw: &str) -> String {
    let stripped = REASONING.strip(raw);
    // Nothing outside the block means the model put the whole message inside
    // it. Keep the text and drop the tags rather than losing the message.
    let body = if stripped.trim().is_empty() {
        REASONING.unwrap(raw)
    } else {
        stripped
    };
    let body = ANSWER_TAGS.replace_all(&body, "");
    strip_code_fence(body.trim()).trim().to_string()
}

/// Parse all `<tool_call>` blocks from a model response.
///
/// Reasoning blocks are ignored — see [`strip_thinking`].
///
/// Returns an empty vec if the response contains no tool calls (i.e. it is the
/// final answer).
/// The parameter a tool's element *body* maps to, in the element-style dialect.
///
/// `<write_file path="a.py">…</write_file>` puts the path in an attribute and
/// the content in the body, so each tool needs to know which of its parameters
/// the body is. Tools absent from this list take attributes only.
fn body_param(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "write_file" => "content",
        "apply_patch" => "patch",
        "bash" => "command",
        "think" => "thought",
        "plan_task" => "steps",
        "task_complete" => "summary",
        "record_memory" => "value",
        "search_files" | "web_search" => "query",
        _ => return None,
    })
}

/// Parse `name="value"` pairs off an element's attribute list.
fn parse_attrs(raw: &str) -> Vec<(String, String)> {
    static ATTRS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"([A-Za-z_][\w.-]*)\s*=\s*"([^"]*)""#).expect("hardcoded regex is valid")
    });
    ATTRS
        .captures_iter(raw)
        .map(|c| (c[1].to_string(), c[2].trim().to_string()))
        .collect()
}

/// Re-render an element-style call as the `<tool_call>` markup the canonical
/// parser understands, so both dialects converge on one implementation.
fn element_to_canonical(tool: &str, attrs: &str, body: Option<&str>) -> Option<ToolCall> {
    let mut params: Vec<(String, String)> = parse_attrs(attrs);
    // Common aliases models reach for.
    for (from, to) in [("file_path", "path"), ("file", "path"), ("cmd", "command")] {
        if let Some(i) = params.iter().position(|(k, _)| k == from) {
            if !params.iter().any(|(k, _)| k == to) {
                params[i].0 = to.to_string();
            }
        }
    }
    if let (Some(param), Some(text)) = (body_param(tool), body) {
        let text = text.trim();
        if !text.is_empty() && !params.iter().any(|(k, _)| k == param) {
            params.push((param.to_string(), text.to_string()));
        }
    }
    let rendered = params
        .iter()
        .map(|(k, v)| format!("<{k}>{v}</{k}>"))
        .collect::<String>();
    parse_single_tool(tool, &rendered)
}

/// Element-style calls: `<write_file path="a.py">body</write_file>` and the
/// self-closing `<read_file path="a.py"/>`.
///
/// This is the shape VibeCoder's own chat prompt teaches (`commands.rs`), and
/// models carry it into agent runs. Only names in [`AVAILABLE_TOOL_NAMES`] are
/// considered, so ordinary markup in prose — `<div>`, `<p>` — is never mistaken
/// for a call.
/// The paired and self-closing element regexes, one pair per tool, built once.
///
/// These used to be compiled inside `parse_element_calls`, twice per tool name
/// per call — 28 regex compilations every time a model response was parsed, and
/// the count grew with every tool added. Building them once makes the parse
/// cost independent of how many tools exist, which is the property
/// `cost_does_not_scale_with_the_number_of_tools` in
/// `tests/tool_parse_allocations.rs` holds us to.
fn element_patterns() -> &'static [(&'static str, Regex, Regex)] {
    static PATTERNS: LazyLock<Vec<(&'static str, Regex, Regex)>> = LazyLock::new(|| {
        AVAILABLE_TOOL_NAMES
            .iter()
            .map(|name| {
                let paired = Regex::new(&format!(r"(?s)<{name}(\s[^>]*?)?>(.*?)</{name}\s*>"))
                    .expect("generated regex is valid");
                let solo = Regex::new(&format!(r"<{name}(\s[^>]*?)?/>"))
                    .expect("generated regex is valid");
                (*name, paired, solo)
            })
            .collect()
    });
    &PATTERNS
}

fn parse_element_calls(text: &str) -> Vec<ToolCall> {
    // One regex per tool name rather than one alternation with a backreference:
    // Rust's `regex` has no backreferences, so `</\1>` cannot express "the same
    // tag we opened". Matching each name explicitly is exact and cheap at 14
    // tools. Positions are kept so calls come back in document order — a model
    // that writes a file then runs it must have them executed that way round.
    let mut found: Vec<(usize, ToolCall)> = Vec::new();
    let mut consumed: Vec<(usize, usize)> = Vec::new();

    for (name, paired, _) in element_patterns() {
        for c in paired.captures_iter(text) {
            let whole = c.get(0).expect("group 0 always exists");
            if let Some(call) = element_to_canonical(
                name,
                c.get(1).map_or("", |m| m.as_str()),
                Some(c.get(2).map_or("", |m| m.as_str())),
            ) {
                found.push((whole.start(), call));
                consumed.push((whole.start(), whole.end()));
            }
        }
    }

    // Self-closing form, ignoring anything already inside a paired match.
    for (name, _, solo) in element_patterns() {
        for c in solo.captures_iter(text) {
            let whole = c.get(0).expect("group 0 always exists");
            if consumed
                .iter()
                .any(|(s, e)| whole.start() >= *s && whole.end() <= *e)
            {
                continue;
            }
            if let Some(call) =
                element_to_canonical(name, c.get(1).map_or("", |m| m.as_str()), None)
            {
                found.push((whole.start(), call));
            }
        }
    }

    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, call)| call).collect()
}

/// JSON-object calls: `{"name": "read_file", "arguments": {"path": "x"}}`,
/// with or without a surrounding code fence.
///
/// Observed live from minimax-m3 when it is given no native tool definitions —
/// it reaches for the OpenAI function-call shape it was trained on.
fn parse_json_calls(text: &str) -> Vec<ToolCall> {
    static JSON_CALL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)\{[^{}]*"name"\s*:\s*"([^"]+)"[^{}]*"arguments"\s*:\s*(\{.*?\})\s*\}"#)
            .expect("hardcoded regex is valid")
    });
    JSON_CALL
        .captures_iter(text)
        .filter_map(|c| {
            let tool = c[1].trim();
            if !AVAILABLE_TOOL_NAMES.contains(&tool) {
                return None;
            }
            let args: serde_json::Value = serde_json::from_str(&c[2]).ok()?;
            let obj = args.as_object()?;
            let mut params: Vec<(String, String)> = obj
                .iter()
                .map(|(k, v)| {
                    let s = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    (k.clone(), s)
                })
                .collect();
            for (from, to) in [("file_path", "path"), ("file", "path"), ("cmd", "command")] {
                if let Some(i) = params.iter().position(|(k, _)| k == from) {
                    if !params.iter().any(|(k, _)| k == to) {
                        params[i].0 = to.to_string();
                    }
                }
            }
            let rendered = params
                .iter()
                .map(|(k, v)| format!("<{k}>{v}</{k}>"))
                .collect::<String>();
            parse_single_tool(tool, &rendered)
        })
        .collect()
}

pub fn parse_tool_calls(text: &str) -> Vec<ToolCall> {
    let text = &strip_thinking(text);
    // Match <tool_call name="...">...</tool_call> — possibly multi-line
    let outer_re = tool_call_re();

    let mut calls = Vec::new();

    for cap in outer_re.captures_iter(text) {
        let tool_name = cap[1].trim();
        let body = &cap[2];

        if let Some(call) = parse_single_tool(tool_name, body) {
            calls.push(call);
        }
    }

    // Fall back to the other dialects only when the canonical one produced
    // nothing. A turn that already spoke `<tool_call>` is not re-scanned, so a
    // documented example quoted inside a real call cannot double-fire.
    if calls.is_empty() {
        calls = parse_element_calls(text);
    }
    if calls.is_empty() {
        calls = parse_json_calls(text);
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
            let max_steps = extract_tag(body, "max_steps").and_then(|s| s.parse().ok());
            let max_depth = extract_tag(body, "max_depth").and_then(|s| s.parse().ok());
            Some(ToolCall::SpawnAgent {
                task,
                max_steps,
                max_depth,
            })
        }
        "think" => {
            let thought = extract_tag(body, "thought").unwrap_or_default();
            Some(ToolCall::Think { thought })
        }
        "plan_task" => {
            let steps = extract_tag(body, "steps").unwrap_or_default();
            Some(ToolCall::PlanTask { steps })
        }
        "diffstat" => {
            let path = extract_tag(body, "path")?;
            Some(ToolCall::Diffstat { path })
        }
        "record_memory" => {
            let key = extract_tag(body, "key")?;
            let value = extract_tag(body, "value").unwrap_or_default();
            Some(ToolCall::RecordMemory { key, value })
        }
        _ => None,
    }
}

/// The five XML predefined entities, plus the numeric apostrophe models emit
/// interchangeably with `&apos;`. Longest-prefix order is irrelevant here
/// because every key is unambiguous after the leading `&`.
const XML_ENTITIES: &[(&str, char)] = &[
    ("&amp;", '&'),
    ("&lt;", '<'),
    ("&gt;", '>'),
    ("&quot;", '"'),
    ("&apos;", '\''),
    ("&#39;", '\''),
];

/// Decode XML entities in a tool-call parameter.
///
/// The tool protocol is XML-shaped, so models escape `&`, `<` and `>` inside
/// `<content>` — that is what XML *requires* of a text node, and it happens
/// whether or not [`TOOL_SYSTEM_PROMPT`] asks for it. Without this decode the
/// entities are written to disk verbatim and the generated file is not valid
/// source: `&self` arrives as `&amp;self`, `-> Result<T, E>` as
/// `-&gt; Result&lt;T, E&gt;`.
///
/// Single-pass by construction. Sequential `replace` calls would decode
/// `&amp;lt;` — a literal `&lt;` the model escaped correctly — twice, into `<`.
/// Here the scanner consumes `&amp;` and resumes *after* it, so the remaining
/// `lt;` is copied as text and the result is the intended `&lt;`.
///
/// An `&` that starts no known entity (`cargo test 2>&1`) is passed through.
fn decode_xml_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        match XML_ENTITIES
            .iter()
            .find_map(|(entity, ch)| tail.strip_prefix(entity).map(|r| (*ch, r)))
        {
            Some((ch, remainder)) => {
                out.push(ch);
                rest = remainder;
            }
            // Not an entity — emit the bare `&` and continue past it.
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Escape a value for an XML text node. Inverse of [`decode_xml_entities`].
///
/// `&` must go first or the later replacements are re-escaped.
fn escape_xml_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Byte index just past the first `<tag>` (or `</tag>` when `closing`) in
/// `body`, or `None`.
///
/// A plain scan rather than a regex: the pattern has no metacharacters, and the
/// caller runs once per parameter of every tool call. Uses `get` rather than
/// slicing so a non-ASCII tag can only fail to match, never panic on a char
/// boundary.
fn find_tag_end(body: &str, tag: &str, closing: bool) -> Option<usize> {
    let mut from = 0usize;
    while let Some(rel) = body.get(from..)?.find('<') {
        let at = from + rel;
        let after_lt = at + 1;
        let rest = body.get(after_lt..)?;
        let rest = if closing {
            match rest.strip_prefix('/') {
                Some(r) => r,
                None => {
                    from = after_lt;
                    continue;
                }
            }
        } else if rest.starts_with('/') {
            from = after_lt;
            continue;
        } else {
            rest
        };
        if let Some(after_name) = rest.strip_prefix(tag) {
            if after_name.starts_with('>') {
                // Distance from `at` to just past the '>' that closes this tag.
                let consumed = body.len() - after_name.len() + 1;
                return Some(consumed);
            }
        }
        from = after_lt;
    }
    None
}

/// Extract content from `<tag>...</tag>` in a body string, decoding any XML
/// entities the model escaped (see [`decode_xml_entities`]).
fn extract_tag(body: &str, tag: &str) -> Option<String> {
    let start = find_tag_end(body, tag, false)?;
    let rest = body.get(start..)?;
    // The close is located within `rest`, so a `</tag>` appearing *before* the
    // open cannot terminate the match.
    let close_end = find_tag_end(rest, tag, true)?;
    let inner = rest.get(..close_end.checked_sub(tag.len() + 3)?)?;
    Some(decode_xml_entities(inner.trim())).filter(|s| !s.is_empty())
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
        assert!(
            matches!(&calls[0], ToolCall::TaskComplete { summary } if summary.contains("Done"))
        );
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
        assert!(
            matches!(&calls[0], ToolCall::SearchFiles { query, glob: Some(g) }
            if query == "fn main" && g == "*.rs")
        );
    }

    // ── ToolCall::name() ─────────────────────────────────────────────────

    #[test]
    fn tool_call_names() {
        assert_eq!(ToolCall::ReadFile { path: "a".into() }.name(), "read_file");
        assert_eq!(
            ToolCall::WriteFile {
                path: "a".into(),
                content: "b".into()
            }
            .name(),
            "write_file"
        );
        assert_eq!(
            ToolCall::ApplyPatch {
                path: "a".into(),
                patch: "b".into()
            }
            .name(),
            "apply_patch"
        );
        assert_eq!(
            ToolCall::Bash {
                command: "ls".into()
            }
            .name(),
            "bash"
        );
        assert_eq!(
            ToolCall::SearchFiles {
                query: "q".into(),
                glob: None
            }
            .name(),
            "search_files"
        );
        assert_eq!(
            ToolCall::ListDirectory { path: ".".into() }.name(),
            "list_directory"
        );
        assert_eq!(
            ToolCall::WebSearch {
                query: "q".into(),
                num_results: 5
            }
            .name(),
            "web_search"
        );
        assert_eq!(ToolCall::FetchUrl { url: "u".into() }.name(), "fetch_url");
        assert_eq!(
            ToolCall::TaskComplete {
                summary: "s".into()
            }
            .name(),
            "task_complete"
        );
        assert_eq!(
            ToolCall::SpawnAgent {
                task: "t".into(),
                max_steps: None,
                max_depth: None
            }
            .name(),
            "spawn_agent"
        );
    }

    // ── ToolCall::is_destructive() ───────────────────────────────────────

    #[test]
    fn is_destructive_true_for_bash() {
        assert!(ToolCall::Bash {
            command: "rm -rf /".into()
        }
        .is_destructive());
    }

    #[test]
    fn is_destructive_true_for_write() {
        assert!(ToolCall::WriteFile {
            path: "a".into(),
            content: "b".into()
        }
        .is_destructive());
    }

    #[test]
    fn is_destructive_true_for_patch() {
        assert!(ToolCall::ApplyPatch {
            path: "a".into(),
            patch: "b".into()
        }
        .is_destructive());
    }

    #[test]
    fn is_destructive_true_for_spawn() {
        assert!(ToolCall::SpawnAgent {
            task: "t".into(),
            max_steps: None,
            max_depth: None
        }
        .is_destructive());
    }

    #[test]
    fn is_destructive_false_for_read() {
        assert!(!ToolCall::ReadFile { path: "a".into() }.is_destructive());
        assert!(!ToolCall::SearchFiles {
            query: "q".into(),
            glob: None
        }
        .is_destructive());
        assert!(!ToolCall::ListDirectory { path: ".".into() }.is_destructive());
        assert!(!ToolCall::WebSearch {
            query: "q".into(),
            num_results: 5
        }
        .is_destructive());
        assert!(!ToolCall::FetchUrl { url: "u".into() }.is_destructive());
        assert!(!ToolCall::TaskComplete {
            summary: "done".into()
        }
        .is_destructive());
    }

    // ── ToolCall::is_terminal() ──────────────────────────────────────────

    #[test]
    fn is_terminal_only_for_task_complete() {
        assert!(ToolCall::TaskComplete {
            summary: "done".into()
        }
        .is_terminal());
        assert!(!ToolCall::ReadFile { path: "a".into() }.is_terminal());
        assert!(!ToolCall::Bash {
            command: "ls".into()
        }
        .is_terminal());
    }

    // ── ToolCall::summary() ──────────────────────────────────────────────

    #[test]
    fn summary_read_file() {
        let s = ToolCall::ReadFile {
            path: "/src/main.rs".into(),
        }
        .summary();
        assert_eq!(s, "read_file(/src/main.rs)");
    }

    #[test]
    fn summary_write_file_counts_lines() {
        let s = ToolCall::WriteFile {
            path: "a.rs".into(),
            content: "line1\nline2\nline3\n".into(),
        }
        .summary();
        assert!(s.contains("3 lines"), "got: {}", s);
    }

    #[test]
    fn summary_apply_patch_counts_hunks() {
        let patch = "@@ -1,3 +1,4 @@\n foo\n+bar\n@@ -10,2 +11,3 @@\n baz\n+qux\n";
        let s = ToolCall::ApplyPatch {
            path: "a.rs".into(),
            patch: patch.into(),
        }
        .summary();
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
        let s = ToolCall::SearchFiles {
            query: "foo".into(),
            glob: Some("*.rs".into()),
        }
        .summary();
        assert!(s.contains("*.rs"), "got: {}", s);
    }

    #[test]
    fn summary_search_without_glob() {
        let s = ToolCall::SearchFiles {
            query: "bar".into(),
            glob: None,
        }
        .summary();
        assert!(s.contains("bar") && !s.contains("in"), "got: {}", s);
    }

    #[test]
    fn summary_spawn_agent() {
        let s = ToolCall::SpawnAgent {
            task: "do stuff".into(),
            max_steps: Some(5),
            max_depth: Some(2),
        }
        .summary();
        assert!(s.contains("max_steps=5"), "got: {}", s);
        assert!(s.contains("max_depth=2"), "got: {}", s);
    }

    #[test]
    fn summary_spawn_agent_defaults() {
        let s = ToolCall::SpawnAgent {
            task: "x".into(),
            max_steps: None,
            max_depth: None,
        }
        .summary();
        assert!(
            s.contains("max_steps=10"),
            "default should be 10, got: {}",
            s
        );
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
        let call = ToolCall::ReadFile {
            path: "a.rs".into(),
        };
        let result = ToolResult {
            tool_name: "read_file".into(),
            output: "fn main() {}".into(),
            success: true,
            truncated: false,
        };
        let formatted = format_tool_result(&call, &result);
        assert!(formatted.starts_with("✅"));
        assert!(formatted.contains("read_file"));
        assert!(formatted.contains("fn main()"));
    }

    #[test]
    fn format_tool_result_error() {
        let call = ToolCall::Bash {
            command: "bad".into(),
        };
        let result = ToolResult::err("bash", "not found");
        let formatted = format_tool_result(&call, &result);
        assert!(formatted.starts_with("❌"));
    }

    #[test]
    fn format_tool_result_truncated_note() {
        let call = ToolCall::Bash {
            command: "cat big".into(),
        };
        let result = ToolResult {
            tool_name: "bash".into(),
            output: "data".into(),
            success: true,
            truncated: true,
        };
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
        assert!(
            matches!(&calls[0], ToolCall::WebSearch { query, num_results: 3 } if query == "rust async")
        );
    }

    #[test]
    fn parse_web_search_default_num_results() {
        let text = r#"<tool_call name="web_search">
<query>hello</query>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert!(matches!(
            &calls[0],
            ToolCall::WebSearch { num_results: 5, .. }
        ));
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
        assert!(
            matches!(&calls[0], ToolCall::SpawnAgent { task, max_steps: Some(5), max_depth: Some(2) } if task == "Write tests")
        );
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

#[cfg(test)]
mod thinking_tests {
    use super::*;

    // Models quote the call they are considering inside their reasoning —
    // gpt-oss does it verbatim. Executing those would run tools the model
    // never actually invoked (and, worse, ones it decided against).

    /// Every advertised tool must exist, and its schema's parameter names must
    /// be the ones the parser extracts. A native call is transcribed to XML by
    /// `render_tool_call` and re-parsed, so a single renamed key silently turns
    /// every native tool call into an unparsed block.
    /// A reasoning-only turn whose reasoning *is* the reply: strip yields
    /// nothing, so the display path unwraps rather than showing an empty turn.
    /// Taken verbatim from a minimax-m3 turn that put `</mm:think>` on screen
    /// mid-sentence: the provider had eaten the opening tag into its own
    /// reasoning field, leaving a namespaced orphan close we did not recognise.
    /// The element dialect VibeCoder's own chat prompt teaches, verbatim from
    /// the turn that rendered raw on screen.
    #[test]
    fn element_dialect_write_file_is_parsed() {
        let text = "<write_file path=\"fibonacci.py\">def fib(n):\n    return n</write_file>";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1, "got {calls:?}");
        match &calls[0] {
            ToolCall::WriteFile { path, content } => {
                assert_eq!(path, "fibonacci.py");
                assert!(
                    content.contains("def fib"),
                    "body became the content: {content:?}"
                );
            }
            other => panic!("expected WriteFile, got {other:?}"),
        }
    }

    #[test]
    fn element_dialect_self_closing_and_aliases() {
        let calls = parse_tool_calls("<read_file file_path=\"README.md\" />");
        assert_eq!(calls.len(), 1, "got {calls:?}");
        assert!(matches!(&calls[0], ToolCall::ReadFile { path } if path == "README.md"));
    }

    /// Captured live from minimax-m3 given no native tools: it falls back to
    /// the OpenAI function-call shape.
    #[test]
    fn json_dialect_is_parsed() {
        let text = "I'll read it.\n```\n{\"name\": \"read_file\", \"arguments\": {\"file_path\": \"README.md\"}}\n```";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1, "got {calls:?}");
        assert!(matches!(&calls[0], ToolCall::ReadFile { path } if path == "README.md"));
    }

    /// The canonical dialect must still win, and must not be re-scanned — a
    /// `<tool_call>` body mentioning `<write_file>` would otherwise double-fire.
    #[test]
    fn canonical_dialect_takes_precedence_and_is_not_rescanned() {
        let text =
            "<tool_call name=\"write_file\"><path>a.py</path><content>x</content></tool_call>";
        assert_eq!(parse_tool_calls(text).len(), 1);
    }

    /// Ordinary markup and prose must never be mistaken for a call.
    #[test]
    fn html_and_prose_are_not_tool_calls() {
        for text in [
            "<div>hello</div><p>world</p>",
            "Use `write_file` to save it.",
            "<span class=\"bash\">not a command</span>",
            "{\"name\": \"not_a_tool\", \"arguments\": {\"x\": 1}}",
        ] {
            assert!(
                parse_tool_calls(text).is_empty(),
                "false positive on {text:?}: {:?}",
                parse_tool_calls(text)
            );
        }
    }

    /// Reasoning quotes calls the model then rejects — every dialect must be
    /// read from the visible turn only.
    #[test]
    fn dialects_inside_reasoning_are_ignored() {
        let text = "<thinking>I could <write_file path=\"x\">bad</write_file></thinking>Done.";
        assert!(
            parse_tool_calls(text).is_empty(),
            "{:?}",
            parse_tool_calls(text)
        );
    }

    #[test]
    fn namespaced_orphan_close_is_stripped() {
        let turn = "Let me write a single Python file.</mm:think>def fib(n): pass";
        let visible = strip_thinking(turn);
        assert!(
            !visible.contains("mm:think"),
            "namespaced tag leaked: {visible:?}"
        );
        assert!(
            !visible.contains("Let me write"),
            "reasoning before an orphan close is still reasoning: {visible:?}"
        );
        assert!(
            visible.contains("def fib"),
            "the answer was dropped: {visible:?}"
        );
    }

    #[test]
    fn namespaced_blocks_and_unclosed_openers_are_stripped() {
        assert_eq!(
            strip_thinking("<mm:think>plan</mm:think>answer").trim(),
            "answer"
        );
        assert_eq!(
            strip_thinking("<ns:thinking>plan</ns:thinking>answer").trim(),
            "answer"
        );
        assert_eq!(strip_thinking("answer<mm:think>cut off").trim(), "answer");
    }

    #[test]
    fn unwrap_thinking_handles_namespaced_tags() {
        assert_eq!(
            unwrap_thinking("<mm:think>the answer</mm:think>"),
            "the answer"
        );
    }

    /// A plain closing angle bracket in prose must not be mistaken for a tag.
    #[test]
    fn ordinary_prose_survives_the_orphan_rule() {
        let text = "Use a < b and check </div> in the template.";
        assert_eq!(strip_thinking(text), text);
    }

    #[test]
    fn unwrap_thinking_keeps_the_text_and_drops_the_tags() {
        let whole = "<thinking>Here is the program:\n```py\nprint(1)\n```</thinking>";
        assert_eq!(strip_thinking(whole).trim(), "");
        let unwrapped = unwrap_thinking(whole);
        assert!(unwrapped.starts_with("Here is the program:"));
        assert!(!unwrapped.contains("<thinking>"));
        assert!(!unwrapped.contains("</thinking>"));
        assert!(unwrapped.contains("print(1)"));
    }

    #[test]
    fn unwrap_thinking_leaves_untagged_text_alone() {
        assert_eq!(unwrap_thinking("just an answer"), "just an answer");
        assert_eq!(unwrap_thinking("<think>a</think>b"), "ab");
    }

    // ── Commit-message sanitising ────────────────────────────────────────────
    //
    // The inputs below are shortened from two commit subjects a user reported
    // (b846b47, ba57e7b in their repo): the model's whole deliberation, tags
    // and all, was committed verbatim.

    #[test]
    fn commit_message_drops_a_leading_thinking_block() {
        let raw = "<thinking>```\nx Pin GitHub Actions\n```\nWait, let's refine. \
Subject: Pin GitHub Actions and escape HTML in benchmark reports\n\
Let's output the message.</thinking>\n\
Pin GitHub Actions and escape HTML in benchmark reports\n\n\
- Pin action versions to specific commit SHAs";
        assert_eq!(
            sanitize_commit_message(raw),
            "Pin GitHub Actions and escape HTML in benchmark reports\n\n\
- Pin action versions to specific commit SHAs"
        );
    }

    #[test]
    fn commit_message_keeps_the_text_when_the_block_holds_everything() {
        // ba57e7b: the model repeated itself, so stripping the block left the
        // message — but a model that emits *only* the block must not produce
        // an empty commit message.
        assert_eq!(
            sanitize_commit_message("<thinking>Add .vibecoder to .gitignore</thinking>"),
            "Add .vibecoder to .gitignore"
        );
        assert_eq!(
            sanitize_commit_message(
                "<thinking>Add .vibecoder to .gitignore</thinking> Add .vibecoder to .gitignore"
            ),
            "Add .vibecoder to .gitignore"
        );
    }

    #[test]
    fn commit_message_handles_the_other_reasoning_spellings() {
        for raw in [
            "<think>deliberating</think>Fix the parser",
            "<mm:think>deliberating</mm:think>Fix the parser",
            "<reasoning>deliberating</reasoning>\nFix the parser",
            "<analysis>deliberating</analysis>Fix the parser",
            "<scratchpad>deliberating</scratchpad>Fix the parser",
            "<reflection>deliberating</reflection>Fix the parser",
            // Orphan close: the provider ate the opening tag.
            "deliberating</think>Fix the parser",
        ] {
            assert_eq!(
                sanitize_commit_message(raw),
                "Fix the parser",
                "input: {raw}"
            );
        }
    }

    #[test]
    fn commit_message_drops_an_unclosed_reasoning_tag() {
        // The stream ended mid-block, so there is no closing tag to match on.
        // The message still has to reach the repository without the tag —
        // subjects reading "<thinking>Fix the parser" have been committed.
        assert_eq!(
            sanitize_commit_message("<thinking>Fix the parser"),
            "Fix the parser"
        );
        assert_eq!(
            sanitize_commit_message("Fix the parser\n\n<thinking>or should it be"),
            "Fix the parser"
        );
    }

    #[test]
    fn commit_message_unwraps_answer_tags_and_a_whole_reply_fence() {
        assert_eq!(
            sanitize_commit_message("<answer>Fix the parser</answer>"),
            "Fix the parser"
        );
        assert_eq!(
            sanitize_commit_message("```\nFix the parser\n```"),
            "Fix the parser"
        );
        assert_eq!(
            sanitize_commit_message("```text\nFix the parser\n\n- one bullet\n```"),
            "Fix the parser\n\n- one bullet"
        );
    }

    #[test]
    fn commit_message_leaves_a_clean_message_alone() {
        let clean = "Fix the parser\n\n- one bullet\n- another <T> generic mention";
        assert_eq!(sanitize_commit_message(clean), clean);
    }

    #[test]
    fn commit_message_is_empty_when_the_reply_held_none() {
        // Callers must fail rather than commit this.
        assert_eq!(sanitize_commit_message("<thinking>"), "");
        assert_eq!(sanitize_commit_message("   \n  "), "");
    }

    #[test]
    fn tool_definitions_match_parser() {
        let defs = tool_definitions();
        assert_eq!(
            defs.len(),
            AVAILABLE_TOOL_NAMES.len(),
            "every advertised tool needs a schema"
        );

        for def in &defs {
            let f = &def["function"];
            let name = f["name"].as_str().expect("tool name");
            assert!(
                AVAILABLE_TOOL_NAMES.contains(&name),
                "{name} is advertised but not in AVAILABLE_TOOL_NAMES"
            );

            // Build a native call using every required param, then round-trip
            // it exactly as a provider would.
            let required: Vec<&str> = f["parameters"]["required"]
                .as_array()
                .expect("required array")
                .iter()
                .map(|v| v.as_str().expect("required entry"))
                .collect();
            let props = f["parameters"]["properties"]
                .as_object()
                .expect("properties object");
            for r in &required {
                assert!(
                    props.contains_key(*r),
                    "{name}: required `{r}` has no schema"
                );
            }

            let args: serde_json::Map<String, serde_json::Value> = required
                .iter()
                .map(|r| {
                    let ty = props[*r]["type"].as_str().unwrap_or("string");
                    // Deliberately carries the three XML metacharacters, to pin
                    // `render_tool_call` and `extract_tag` as inverses. With a
                    // bare `"x"` the pair could drift out of symmetry — escape
                    // without decode, or decode without escape — and every
                    // native tool call carrying a `&` or `<` would be mangled.
                    let v = if ty == "integer" {
                        serde_json::json!(3)
                    } else {
                        serde_json::json!("fn f(x: &T) -> Result<U, E> { x < 1 }")
                    };
                    ((*r).to_string(), v)
                })
                .collect();

            let xml = render_tool_call(name, Some(&serde_json::Value::Object(args)));
            let parsed = parse_tool_calls(&xml);
            assert_eq!(
                parsed.len(),
                1,
                "{name}: native call did not round-trip through the parser: {xml}"
            );

            // Call count alone would not notice a value being mangled on the
            // way through, so check the payload of the one tool that carries a
            // whole file body — that is where the corruption actually landed.
            if let [ToolCall::WriteFile { content, .. }] = parsed.as_slice() {
                assert_eq!(
                    content, "fn f(x: &T) -> Result<U, E> { x < 1 }",
                    "write_file content did not survive render → parse"
                );
            }
        }
    }

    // ── XML entity handling in tool parameters ──────────────────────────────
    //
    // Regression cover for escaped `&`/`<`/`>` reaching disk verbatim, which
    // made every generated Rust file with a reference or a generic invalid.

    #[test]
    fn write_file_content_decodes_xml_entities() {
        let text = "<tool_call name=\"write_file\">\
                    <path>src/db.rs</path>\
                    <content>pub fn pool(&amp;self) -&gt; &amp;Pool { &amp;self.pool }</content>\
                    </tool_call>";
        let calls = parse_tool_calls(text);
        match calls.as_slice() {
            [ToolCall::WriteFile { path, content }] => {
                assert_eq!(path, "src/db.rs");
                assert_eq!(content, "pub fn pool(&self) -> &Pool { &self.pool }");
            }
            other => panic!("expected one write_file, got {other:?}"),
        }
    }

    #[test]
    fn generic_type_params_survive_decoding() {
        let text = "<tool_call name=\"write_file\">\
                    <path>a.rs</path>\
                    <content>async fn new() -&gt; Result&lt;Self, sqlx::Error&gt; {}</content>\
                    </tool_call>";
        match parse_tool_calls(text).as_slice() {
            [ToolCall::WriteFile { content, .. }] => {
                assert_eq!(content, "async fn new() -> Result<Self, sqlx::Error> {}");
            }
            other => panic!("expected one write_file, got {other:?}"),
        }
    }

    #[test]
    fn double_escaped_entity_decodes_once() {
        // A model correctly escaping a literal `&lt;` for an HTML file sends
        // `&amp;lt;`. Sequential string replaces would yield `<`.
        assert_eq!(decode_xml_entities("&amp;lt;p&amp;gt;"), "&lt;p&gt;");
    }

    #[test]
    fn bare_ampersand_is_not_mangled() {
        // `2>&1` is not an entity — the shell redirect must survive intact.
        assert_eq!(
            decode_xml_entities("cargo test 2>&1 | head -50"),
            "cargo test 2>&1 | head -50"
        );
        assert_eq!(decode_xml_entities("a && b"), "a && b");
    }

    #[test]
    fn quotes_and_backslashes_are_left_alone() {
        // Only `&`, `<`, `>` are escaped by the contract; a raw-string literal
        // and a single-quoted SQL default must round-trip untouched.
        let s = "r#\"DEFAULT 'draft'\"# \\n";
        assert_eq!(decode_xml_entities(s), s);
    }

    #[test]
    fn escape_decode_round_trips() {
        let original = "impl<T> Foo<T> { fn f(&self) -> &T { &self.0 } } // a & b";
        assert_eq!(decode_xml_entities(&escape_xml_text(original)), original);
    }

    // ── Rejection messages ──────────────────────────────────────────────────

    #[test]
    fn a_malformed_known_tool_is_not_called_unavailable() {
        // The observed failure: `<command>pwd</arg_value>`. `bash` is real; the
        // closing tag is not. The old message claimed bash was unavailable.
        let text = "<tool_call name=\"bash\"><command>pwd</arg_value></tool_call>";
        let reason = tool_call_rejection_reason(text).expect("should reject");
        assert!(
            reason.contains("malformed"),
            "should name the real fault: {reason}"
        );
        assert!(
            !reason.contains("is not an available tool"),
            "must not tell the model a valid tool does not exist: {reason}"
        );
        assert!(
            reason.contains("<command>…</command>"),
            "should quote the tags bash needs: {reason}"
        );
    }

    #[test]
    fn an_unknown_tool_is_still_reported_as_unknown() {
        let text = "<tool_call name=\"container.exec\"><cmd>ls</cmd></tool_call>";
        let reason = tool_call_rejection_reason(text).expect("should reject");
        assert!(reason.contains("is not an available tool"), "{reason}");
        assert!(
            reason.contains("bash"),
            "should list the real tools: {reason}"
        );
    }

    #[test]
    fn a_well_formed_call_is_not_rejected() {
        let text = "<tool_call name=\"bash\"><command>pwd</command></tool_call>";
        assert_eq!(tool_call_rejection_reason(text), None);
    }

    #[test]
    fn plain_prose_is_not_rejected() {
        assert_eq!(
            tool_call_rejection_reason("Just explaining the plan."),
            None
        );
    }

    #[test]
    fn every_tool_name_has_a_definition() {
        let defs = tool_definitions();
        for name in AVAILABLE_TOOL_NAMES {
            assert!(
                defs.iter().any(|d| d["function"]["name"] == *name),
                "{name} is in AVAILABLE_TOOL_NAMES but has no schema — a native \
                 tool-calling model would never be told it exists"
            );
        }
    }

    #[test]
    fn tool_call_inside_thinking_is_ignored() {
        let text = r#"<thinking>I could <tool_call name="bash"><command>rm -rf /</command></tool_call> but no.</thinking>Done."#;
        assert!(parse_tool_calls(text).is_empty());
    }

    #[test]
    fn think_spelling_is_also_ignored() {
        let text = r#"<think><tool_call name="read_file"><path>a.rs</path></tool_call></think>"#;
        assert!(parse_tool_calls(text).is_empty());
    }

    #[test]
    fn real_call_after_thinking_still_parses() {
        let text = r#"<thinking>maybe <tool_call name="bash"><command>ls</command></tool_call></thinking>
<tool_call name="list_directory"><path>src</path></tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name(), "list_directory");
    }

    #[test]
    fn unclosed_thinking_block_discards_its_tail() {
        let text =
            r#"prose <thinking>I will <tool_call name="bash"><command>ls</command></tool_call>"#;
        assert!(parse_tool_calls(text).is_empty());
    }

    #[test]
    fn strip_thinking_keeps_surrounding_prose() {
        assert_eq!(strip_thinking("a<thinking>x</thinking>b"), "ab");
    }
}

#[cfg(test)]
mod unknown_tool_tests {
    use super::*;

    // A `<tool_call>` naming a tool we don't have (gpt-oss reaches for its
    // built-in `container.exec`) used to parse to nothing and end the run with
    // the raw markup as its summary.

    #[test]
    fn unknown_tool_name_is_reported() {
        let text = r#"<tool_call name="container.exec"><cmd>ls</cmd></tool_call>"#;
        assert_eq!(
            unparsed_tool_call_name(text).as_deref(),
            Some("container.exec")
        );
    }

    #[test]
    fn missing_required_parameter_is_reported() {
        // read_file without <path> cannot be built.
        let text = r#"<tool_call name="read_file"><file>a.rs</file></tool_call>"#;
        assert_eq!(unparsed_tool_call_name(text).as_deref(), Some("read_file"));
    }

    #[test]
    fn valid_call_reports_nothing() {
        let text = r#"<tool_call name="list_directory"><path>.</path></tool_call>"#;
        assert!(unparsed_tool_call_name(text).is_none());
    }

    #[test]
    fn plain_prose_reports_nothing() {
        assert!(unparsed_tool_call_name("Here is my answer.").is_none());
    }

    #[test]
    fn unknown_tool_inside_thinking_is_not_reported() {
        let text = r#"<thinking><tool_call name="container.exec"><cmd>ls</cmd></tool_call></thinking>Done."#;
        assert!(unparsed_tool_call_name(text).is_none());
    }

    #[test]
    fn unclosed_block_is_reported_as_malformed() {
        let text = r#"<tool_call name="bash"><command>ls"#;
        assert!(unparsed_tool_call_name(text).is_some());
    }

    #[test]
    fn every_advertised_name_parses() {
        // The list we hand back to the model must not name tools the parser
        // rejects.
        for name in AVAILABLE_TOOL_NAMES {
            assert!(
                TOOL_SYSTEM_PROMPT.contains(&format!("name=\"{name}\"")),
                "{name} is advertised but absent from the tool prompt"
            );
        }
    }
}

#[cfg(test)]
mod bash_risk_tests {
    use super::*;

    // The approval banner shouts "Destructive tool" based on is_destructive().
    // Applying that to `find crates -name "*.rs" | sort` trains the user to
    // click through the warnings that actually matter.

    #[test]
    fn inspection_commands_are_not_destructive() {
        for cmd in [
            "ls -la",
            "find crates -name \"*.rs\" | sort",
            "grep -rn TODO src | head -50",
            "cat Cargo.toml",
            "rg --files | wc -l",
            "git status",
            "git log --oneline -20",
            "git diff HEAD~1",
            "/usr/bin/find . -name '*.rs'",
        ] {
            assert!(
                !ToolCall::Bash {
                    command: cmd.into()
                }
                .is_destructive(),
                "{cmd} should not be flagged destructive"
            );
        }
    }

    #[test]
    fn mutating_commands_stay_destructive() {
        for cmd in [
            "rm -rf target",
            "cargo build",
            "git commit -m x",
            "git push",
            "sed -i '' s/a/b/ f.rs",
            "mv a b",
            "npm install",
        ] {
            assert!(
                ToolCall::Bash {
                    command: cmd.into()
                }
                .is_destructive(),
                "{cmd} must stay flagged destructive"
            );
        }
    }

    #[test]
    fn redirection_and_chaining_defeat_the_allowlist() {
        for cmd in [
            "ls > files.txt",
            "cat a.rs >> b.rs",
            "ls; rm -rf /",
            "ls && rm x",
            "echo `rm -rf x`",
            "echo $(rm -rf x)",
            "find . -name '*.rs' -delete",
            "find . -exec rm {} ;",
        ] {
            assert!(
                ToolCall::Bash {
                    command: cmd.into()
                }
                .is_destructive(),
                "{cmd} must stay flagged destructive"
            );
        }
    }

    #[test]
    fn every_pipeline_segment_must_be_read_only() {
        assert!(!bash_is_read_only("find . -name '*.rs' | xargs rm"));
        assert!(bash_is_read_only("find . -name '*.rs' | sort | head -5"));
    }

    #[test]
    fn writing_git_subcommands_are_not_read_only() {
        assert!(!bash_is_read_only("git commit -am wip"));
        assert!(!bash_is_read_only("git checkout -b feature"));
        assert!(!bash_is_read_only("git reset --hard"));
    }

    #[test]
    fn empty_or_unknown_commands_are_destructive() {
        assert!(!bash_is_read_only(""));
        assert!(!bash_is_read_only("   "));
        assert!(!bash_is_read_only("some-unknown-binary --flag"));
    }

    #[test]
    fn file_mutating_tools_remain_destructive() {
        assert!(ToolCall::WriteFile {
            path: "a".into(),
            content: "b".into()
        }
        .is_destructive());
        assert!(ToolCall::ApplyPatch {
            path: "a".into(),
            patch: "b".into()
        }
        .is_destructive());
        assert!(!ToolCall::ReadFile { path: "a".into() }.is_destructive());
    }
}
