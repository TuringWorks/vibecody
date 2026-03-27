# LangGraph Bridge

Bridge to the Python LangGraph agent ecosystem. Run LangGraph workflows, import LangChain tools, and connect to LangSmith for tracing, all from within VibeCody without leaving your Rust/TypeScript workflow.

## When to Use
- Running existing LangGraph agent workflows from VibeCody
- Importing LangChain tools and chains into the VibeCody agent loop
- Connecting to LangSmith for observability and trace analysis
- Bridging Python ML/AI pipelines with VibeCody coding workflows
- Reusing team LangGraph graphs without rewriting them in Rust

## Commands
- `/langgraph run <graph>` — Execute a LangGraph workflow
- `/langgraph list` — List available LangGraph graphs in the project
- `/langgraph import <tool>` — Import a LangChain tool for agent use
- `/langgraph trace <run-id>` — View LangSmith trace for a run
- `/langgraph serve <graph> <port>` — Serve a graph as an API endpoint
- `/langgraph config` — Configure Python environment and LangSmith API key
- `/langgraph install` — Install langgraph and dependencies in a venv
- `/langgraph status` — Show bridge status and available Python environment

## Examples
```
/langgraph install
# Created venv at .venv, installed langgraph 0.4.2, langchain 0.3.1
# LangSmith API key: configured

/langgraph list
# Found 3 graphs:
# - research_agent (4 nodes, 2 tools)
# - code_review_chain (3 nodes, 1 tool)
# - data_pipeline (6 nodes, 4 tools)

/langgraph run research_agent --input "Find alternatives to Redis for caching"
# Running research_agent...
# Node 1/4: search_web -> 8 results
# Node 2/4: analyze_results -> 3 candidates
# Output: Dragonfly, KeyDB, Garnet — comparison table generated
```

## Best Practices
- Use a dedicated virtual environment to avoid dependency conflicts
- Pin LangGraph and LangChain versions for reproducible workflows
- Enable LangSmith tracing for debugging complex multi-node graphs
- Keep bridge graphs focused on tasks Python excels at (ML, data, NLP)
- Test graphs independently before integrating into VibeCody workflows
