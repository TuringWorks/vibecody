# RL Deployment & Serving

Deploy RL policies with stateful serving, A/B testing, edge deployment, auto-rollback, and domain-specific integrations.

## When to Use
- Deploying RL policies to cloud, edge, embedded, or trading systems
- Setting up stateful policy serving with session management
- Configuring A/B testing between policy versions
- Enabling auto-rollback on reward regression
- Deploying to edge devices via WASM/ONNX runtime
- Integrating with trading engines (FIX protocol) or robotics (ROS 2)

## Commands
- `/rlos deploy <policy> --target cloud` — Deploy to cloud endpoint
- `/rlos deploy <policy> --target edge` — Export for edge deployment
- `/rlos deploy status` — Show active deployments with health
- `/rlos deploy rollback <deployment>` — Rollback to previous policy
- `/rlos deploy ab-test <policy_a> <policy_b> --split 50/50` — Start A/B test
- `/rlos deploy monitor <deployment>` — Live deployment metrics
