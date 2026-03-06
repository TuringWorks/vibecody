---
triggers: ["LangChain", "LangGraph", "chain", "agent langchain", "tool calling", "LangSmith", "LCEL"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# LangChain & LangGraph

When building with LangChain:

1. Use LCEL (LangChain Expression Language) for composing chains: `prompt | model | parser`
2. ChatPromptTemplate: `SystemMessage` + `HumanMessage` with `{variables}` for dynamic content
3. Output parsers: `PydanticOutputParser` for structured JSON, `StrOutputParser` for text
4. Tools: define with `@tool` decorator — name, description, and args schema for LLM tool use
5. Agents: use `create_tool_calling_agent` with tool-capable models (Claude, GPT-4)
6. Memory: `ConversationBufferWindowMemory` (last N messages) or `ConversationSummaryMemory`
7. LangGraph: build stateful multi-step agents as directed graphs — nodes are steps, edges are conditions
8. Retrieval chain: `create_retrieval_chain(retriever, combine_docs_chain)` for RAG
9. Callbacks: use `LangSmith` for tracing, debugging, and evaluating chain runs
10. Streaming: use `.astream()` for async token-by-token output in chat interfaces
11. Document loaders: PDF, HTML, markdown, code — split with `RecursiveCharacterTextSplitter`
12. Caching: use `InMemoryCache` or `SQLiteCache` to avoid redundant LLM calls
