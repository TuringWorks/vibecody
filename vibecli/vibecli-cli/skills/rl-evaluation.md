---
name: "RL Evaluation"
description: "RL Evaluation: Evaluate RL policies with scenario-based testing, off-policy evaluation, safety constraint checking, adversarial robustness, and regression detection. Use when the task involves RL evaluation, off-policy evaluation, policy safety constraints, adversarial robustness."
category: ai
triggers: ["RL evaluation", "off-policy evaluation", "policy safety constraints", "adversarial robustness"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# RL Evaluation

Evaluate RL policies with scenario-based testing, off-policy evaluation, safety constraint checking, adversarial robustness, and regression detection.

## When to Use
- Running evaluation suites with multiple scenarios and metrics
- Performing off-policy evaluation (FQE, importance sampling, doubly robust)
- Checking safety constraints and detecting near-miss violations
- Testing adversarial robustness (FGSM perturbations)
- Detecting policy regression between versions
- Computing finance-specific metrics (Sharpe, Sortino, max drawdown, VaR)

## Commands
- `/rlos eval run <suite.yaml>` — Run an evaluation suite
- `/rlos eval compare <policy_a> <policy_b>` — Compare two policies
- `/rlos eval safety <policy>` — Run safety evaluation
- `/rlos eval ope <policy> --data <logs.parquet>` — Off-policy evaluation
- `/rlos eval report <run_id>` — Generate evaluation report
