# Cost Router

Smart model routing that selects the optimal AI model based on task complexity, cost, latency, and quality requirements. Routes simple tasks to cheap fast models and complex tasks to capable expensive models, reducing costs by up to 70%.

## When to Use
- Reducing AI API costs without sacrificing quality on hard tasks
- Automatically selecting between local and cloud models based on task type
- Setting per-project or per-team cost budgets with enforcement
- Analyzing cost-per-task breakdowns to optimize spending
- Balancing latency requirements with model capability needs

## Commands
- `/costroute enable` — Enable cost-optimized routing for all requests
- `/costroute disable` — Disable routing and use the default model
- `/costroute budget <amount> <period>` — Set a cost budget (e.g., $50/week)
- `/costroute stats` — Show routing decisions, costs, and savings
- `/costroute rules` — Display current routing rules and model tiers
- `/costroute add-rule <condition> <model>` — Add a custom routing rule
- `/costroute simulate <task>` — Preview which model would be selected and why
- `/costroute models` — List available models with cost and capability ratings

## Examples
```
/costroute enable
# Cost routing enabled. 4 model tiers configured:
# Tier 1 (simple): ollama/llama3 — $0/request
# Tier 2 (medium): claude-haiku — $0.003/request
# Tier 3 (complex): claude-sonnet — $0.015/request
# Tier 4 (expert): claude-opus — $0.075/request

/costroute stats
# This week: 342 requests, $4.12 total (saved $18.40 vs opus-only)
# Routing: 45% tier-1, 30% tier-2, 20% tier-3, 5% tier-4

/costroute simulate "Refactor authentication module"
# Selected: Tier 3 (claude-sonnet) — multi-file refactor, medium complexity
```

## Best Practices
- Start with default tiers and adjust based on quality feedback
- Use local models for autocomplete and simple edits to minimize costs
- Set budget alerts at 80% to avoid surprise overages
- Review routing stats weekly to tune tier boundaries
- Override routing for security-critical tasks that need the best model
