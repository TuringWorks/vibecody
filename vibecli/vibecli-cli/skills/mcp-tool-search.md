---
triggers: ["mcp tool search", "lazy tool schema", "deferred schema", "tool registry", "context reduction", "tool stub", "schema on demand", "MCP tool loading"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# MCP Lazy Tool Schema Loading

When implementing or using deferred MCP tool schema loading to reduce upfront context:

1. **Core Concept** — `ToolRegistry` holds `LazyTool` entries that start as lightweight `ToolStub` objects. Full JSON schemas are only fetched when the model selects a specific tool, achieving up to 85% context reduction for large tool sets.

2. **Registration** — Call `registry.register(ToolStub::new("tool_name", "short description"))` for each tool at startup. Optionally chain `.with_category("cat")` for grouping. This is cheap — no network calls, no JSON parsing.

3. **Compact Context** — Call `registry.stubs_context()` to get a single multi-line string with one `"- tool_name: description"` line per tool. Inject this into the model's system prompt instead of full schemas to save tokens.

4. **On-Demand Schema Loading** — When the model selects a tool by name, call `registry.load_schema(name, schema)` to promote it from `LoadState::Stub` to `LoadState::Loaded`. Supply a `ToolSchema` built with `ToolSchema::new(stub, json_schema_value)`.

5. **Schema Context Injection** — After loading, call `registry.schemas_context(&["tool_a", "tool_b"])` to get the full JSON schemas for only the selected tools. Inject this into the next model turn.

6. **Hit/Miss Tracking** — Call `record_hit()` when a schema is already loaded and `record_miss()` when it requires a fetch. Use `hit_rate()` (returns 0.0 when no calls recorded, not NaN) to monitor cache effectiveness.

7. **Savings Estimation** — `savings_pct(selected_names)` returns `(1 - selected/total) * 100` clamped to [0, 100]. Useful for reporting: selecting 3 tools from 20 saves 85%.

8. **Token Estimation** — `context_token_estimate(names)` gives a rough character/4 estimate for the combined stubs + named schemas. Use for budget checks before injection.

9. **Load State** — Three states: `LoadState::Stub` (default), `LoadState::Loaded` (schema available), `LoadState::Failed(reason)` (set externally if fetch fails). Check with `loaded_count()` for monitoring.

10. **Testing** — Ten unit tests in `mcp_tool_search.rs` cover stub registration, schema loading, context generation, hit rate, and savings percentage. Run with `cargo test -p vibecli --lib -- mcp_tool_search`. The BDD suite is in `mcp_tool_search_bdd.rs`.
