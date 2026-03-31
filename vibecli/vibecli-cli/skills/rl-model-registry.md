# RL Model Registry

Manage RL policies with versioning, lineage tracking, quality gates, promotion workflows, and cross-framework export.

## When to Use
- Registering trained RL policies with RL-specific metadata
- Tracking policy lineage (environment version, reward function, parent policy)
- Enforcing quality gates before registration (min reward, safety pass)
- Managing promotion workflows (staging → canary → production)
- Comparing policy versions with metric diffs and lineage divergence
- Generating model cards with training details and eval results

## Commands
- `/rlos registry list` — List all registered policies
- `/rlos registry info <policy>` — Show metadata, lineage, eval results
- `/rlos registry compare <p1> <p2>` — Compare two policy versions
- `/rlos registry promote <policy> --stage production` — Promote policy
- `/rlos registry search --env <env> --algo <algo>` — Search policies
- `/rlos registry card <policy>` — Generate model card documentation
