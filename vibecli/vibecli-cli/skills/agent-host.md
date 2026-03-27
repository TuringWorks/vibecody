# Agent Host

Multi-agent terminal that hosts multiple AI coding assistants simultaneously. Run Claude Code, Gemini CLI, Aider, and other agents in parallel panes, route tasks to the best agent, and orchestrate collaborative workflows.

## When to Use
- Running multiple AI agents side by side for comparison or specialization
- Routing tasks to the agent best suited for each language or domain
- Getting second opinions on code changes from a different model
- Orchestrating complex workflows where different agents handle different steps
- Benchmarking agent performance on identical tasks

## Commands
- `/agenthost add <agent-type>` — Add an agent (claude, gemini, aider, copilot, custom)
- `/agenthost remove <id>` — Remove an agent from the host
- `/agenthost list` — Show all active hosted agents and their status
- `/agenthost route <task>` — Auto-route a task to the best available agent
- `/agenthost broadcast <message>` — Send a message to all agents simultaneously
- `/agenthost compare <task>` — Run the same task on all agents and diff results
- `/agenthost config <id> <key> <value>` — Configure a specific agent instance

## Examples
```
/agenthost add claude
/agenthost add gemini
/agenthost add aider
# Hosting 3 agents: claude-1 (ready), gemini-1 (ready), aider-1 (ready)

/agenthost route "Optimize this SQL query for PostgreSQL"
# Routed to gemini-1 (best match: SQL optimization, score: 0.89)

/agenthost compare "Write a binary search in Rust"
# claude-1: 14 lines, 3 tests, 0.8s | gemini-1: 18 lines, 2 tests, 1.1s
```

## Best Practices
- Assign agent specializations to improve routing accuracy over time
- Use broadcast sparingly as it consumes tokens across all agents
- Compare mode is ideal for critical code where correctness matters most
- Set per-agent token budgets to control costs in multi-agent setups
- Keep agent count under 5 to maintain manageable terminal output
