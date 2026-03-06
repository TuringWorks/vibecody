---
triggers: ["AI agent", "tool use", "ReAct", "agent loop", "MCP", "function calling", "agent planning"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# AI Agent Development

When building AI agents:

1. ReAct loop: Observe → Think → Act → Observe — iterate until task complete
2. Define clear tools: name, description, parameters schema — the LLM needs to understand when to use each
3. Tool descriptions are prompts: write them like instructions, include examples of when to use
4. Limit tool set per context: 5-15 relevant tools — too many confuse the model
5. Error handling: return clear error messages from tools — the agent needs to recover
6. Planning: for complex tasks, have the agent plan steps before executing
7. Human-in-the-loop: require approval for destructive operations (delete, deploy, send)
8. Context management: include relevant files/state in system prompt — agents need context
9. MCP (Model Context Protocol): use for standardized tool and resource integration
10. Guardrails: validate tool arguments before execution — prevent path traversal, injection
11. Observation: log every thought/action/observation — essential for debugging and improvement
12. Termination: define clear completion criteria — agents should know when to stop
