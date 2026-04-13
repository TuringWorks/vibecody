# auto-approve

Heuristic auto-approval scorer for tool calls. Assigns a risk score (0.0 = safe → 1.0 = dangerous) and emits `AutoApprove`, `AskUser`, or `AutoDeny` without an ML model — using signal-based heuristics.

## Usage

```
/auto-approve <tool_name> <input>
```

## Signals

| Signal | Description | Example triggers |
|---|---|---|
| **Blast Radius** | How wide is the impact? | `rm -rf /`, `DROP DATABASE`, `git push --force` |
| **Irreversibility** | Can the action be undone? | `delete`, `drop`, `truncate`, `shred` |
| **Privilege Escalation** | Does it escalate permissions? | `sudo`, `su`, `chmod 777`, `chown root` |
| **Network Exfiltration** | Does it pipe remote code to a shell? | `curl … \| bash`, `wget -O- \| sh` |

## Decision thresholds (defaults)

| Score range | Decision |
|---|---|
| ≤ 0.20 | `AutoApprove` |
| 0.21 – 0.79 | `AskUser` |
| ≥ 0.80 | `AutoDeny` |

## Known-safe commands (always ≤ 0.05)

`ls`, `cat`, `grep`, `rg`, `find`, `head`, `tail`, `cargo test`, `cargo check`, `cargo clippy`, `git status`, `git log`, `git diff`, `git show`, `echo`, `pwd`, `which`, `wc`, `date`, `whoami`

## Configuration

```rust
let config = ApprovalConfig {
    auto_approve_threshold: 0.2,   // scores at or below → AutoApprove
    auto_deny_threshold: 0.8,      // scores at or above → AutoDeny
    always_allow: vec!["my_safe_tool".to_string()],
    always_deny:  vec!["banned_tool".to_string()],
};
let approver = AutoApprover::new(config);
let score = approver.evaluate("bash", "rm -rf /");
println!("{:?} score={:.2}", score.decision, score.score);
```

## API

```rust
// Core evaluator
pub struct AutoApprover { pub config: ApprovalConfig }

impl AutoApprover {
    pub fn with_defaults() -> Self
    pub fn new(config: ApprovalConfig) -> Self
    pub fn evaluate(&self, tool_name: &str, input: &str) -> ApprovalScore
}

// Composable free functions
pub fn score_blast_radius(input: &str) -> f32
pub fn score_irreversibility(input: &str) -> f32
pub fn has_privilege_escalation(input: &str) -> bool
pub fn has_network_exfiltration(input: &str) -> bool
pub fn is_known_safe(input: &str) -> bool
pub fn aggregate_score(contributions: &[RiskContribution]) -> f32
```

## Output

```
ApprovalScore {
    score: 0.950,
    decision: AutoDeny,
    contributions: [BlastRadius(1.00), Irreversibility(0.80)],
    rationale: "Score=0.900 → AutoDeny | factors: [BlastRadius(1.00), Irreversibility(0.80)]",
}
```
