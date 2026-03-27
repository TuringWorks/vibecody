# A2A Protocol

Agent-to-agent communication using Google's A2A protocol. Publish agent cards, discover peer agents, delegate tasks across organizational boundaries, and coordinate multi-agent workflows with structured message passing.

## When to Use
- Setting up agent-to-agent communication across services
- Publishing an agent card so other agents can discover your capabilities
- Delegating subtasks to specialized agents (code review, testing, deployment)
- Building multi-agent pipelines that span teams or organizations
- Integrating with external A2A-compatible agent ecosystems

## Commands
- `/a2a publish` — Publish this agent's capability card to the directory
- `/a2a discover <query>` — Search for agents matching a capability query
- `/a2a delegate <agent-id> <task>` — Send a task to a remote agent
- `/a2a status <task-id>` — Check status of a delegated task
- `/a2a inbox` — List incoming task requests from other agents
- `/a2a accept <task-id>` — Accept and begin working on an incoming task
- `/a2a reject <task-id> <reason>` — Reject an incoming task with reason
- `/a2a config` — Show current A2A endpoint and authentication settings

## Examples
```
/a2a publish
# Publishes agent card with skills: [code-review, rust, typescript]

/a2a discover "security auditing"
# Found 3 agents: sec-scanner@acme (trust: 0.92), ...

/a2a delegate sec-scanner@acme "Review PR #142 for vulnerabilities"
# Task t-8a3f delegated, estimated completion: 4 min
```

## Best Practices
- Keep agent cards concise with accurate capability descriptions
- Set trust thresholds before accepting tasks from unknown agents
- Use structured task descriptions so receiving agents parse them reliably
- Monitor delegated task timeouts to avoid blocking workflows
- Rotate A2A authentication tokens on a regular schedule
