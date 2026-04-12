# Agent Auto-Scaler

Adjusts agent pool size based on utilization and queue depth. Matches Devin 2.0's auto-scaling.

## Scale-Up Triggers
- Queue depth ≥ `queue_depth_threshold` (default 5)
- Smoothed utilization > `scale_up_threshold` (default 80%)

## Scale-Down Triggers
- Smoothed utilization < `scale_down_threshold` (default 20%)
- No pending tasks
- Cooldown period expired (default 60s)

## Key Types
- **ScalingPolicy** — min/max agents, thresholds, step size, cooldowns
- **AgentAutoScaler** — `evaluate(metrics)` → `ScalingDecision`
- **PoolMetrics** — active_agents, busy_agents, pending_tasks

## Commands
- `/agent scale status` — current pool size and utilization
- `/agent scale policy` — show current scaling policy
- `/agent scale history` — recent scale-up/down events
