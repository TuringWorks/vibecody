# RL Observability

Monitor RL systems with reward drift detection, distributional shift alerts, safety constraint tracking, and multi-agent traces.

## When to Use
- Monitoring reward distribution drift in production deployments
- Detecting observation/action distributional shift
- Tracking safety constraint violations and near-misses
- Analyzing multi-agent communication patterns and per-agent rewards
- Tracking training health (gradient norms, loss convergence, GPU utilization)
- Setting up alerts for RL-specific anomalies

## Commands
- `/rlos monitor <deployment>` — Show live RL metrics dashboard
- `/rlos monitor drift <deployment>` — Check distributional shift
- `/rlos monitor safety <deployment>` — Safety constraint status
- `/rlos monitor alerts` — List active and recent alerts
- `/rlos monitor agents <deployment>` — Multi-agent metrics
- `/rlos monitor cost` — Compute cost tracking and efficiency
