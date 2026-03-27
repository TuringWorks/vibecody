# Agent Trust

Trust scoring system that tracks historical agent accuracy and adjusts review thresholds automatically. High-trust agents get more autonomy while low-trust agents require human review, creating a self-calibrating approval workflow.

## When to Use
- Automating code review approval for consistently accurate agents
- Building trust profiles for different agent types and task categories
- Setting graduated autonomy levels based on historical performance
- Auditing agent decisions to maintain accountability
- Configuring team-wide trust policies for enterprise deployments

## Commands
- `/trust score` — Show current trust scores for all agents and task types
- `/trust history <agent>` — View accuracy history for a specific agent
- `/trust threshold <level>` — Set minimum trust for auto-approval (0.0-1.0)
- `/trust policy <name> <rules>` — Create a named trust policy
- `/trust audit <period>` — Audit agent decisions over a time period
- `/trust reset <agent>` — Reset trust score for an agent to baseline
- `/trust promote <agent>` — Manually promote an agent to higher trust tier
- `/trust config` — Show trust system configuration

## Examples
```
/trust score
# Agent: claude-sonnet
#   Overall: 0.94 | Code edit: 0.96 | Test write: 0.91 | Refactor: 0.93
#   Status: AUTO-APPROVE (threshold: 0.90)
# Agent: ollama-llama3
#   Overall: 0.72 | Code edit: 0.68 | Test write: 0.79 | Refactor: 0.71
#   Status: REVIEW-REQUIRED (threshold: 0.90)

/trust audit last-30-days
# 1,247 decisions audited
# True positive: 93.2% | False positive: 4.1% | Reverted: 2.7%
# Recommendation: Raise threshold for refactor tasks to 0.93

/trust threshold 0.92
# Auto-approval threshold set to 0.92. Affected agents: 1 demoted.
```

## Best Practices
- Start with conservative thresholds and relax as trust builds
- Audit regularly to catch trust score drift from changing codebases
- Use task-specific trust scores rather than a single overall score
- Never set auto-approval threshold below 0.85 for production code
- Reset trust scores when switching to a new model version
