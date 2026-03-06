---
triggers: ["provider API", "LLM integration", "streaming response", "token counting", "model fallback", "Claude API", "OpenAI API"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# AI Model Integration

When integrating LLM provider APIs:

1. Use official SDKs: `anthropic` (Claude), `openai` (OpenAI/compatible), provider-specific clients
2. Streaming: use SSE/streaming endpoints for chat UIs — display tokens as they arrive
3. Token counting: estimate input tokens before sending — avoid exceeding context limits
4. Fallback chain: primary model → fallback model → error — handle rate limits and outages
5. Retry with exponential backoff: 429 (rate limit) → wait 1s, 2s, 4s... with jitter
6. Set timeouts: 30s for chat, 120s for complex tasks — abort and return error message
7. Context window management: truncate old messages, summarize history, use sliding window
8. Cost tracking: log model, input tokens, output tokens per request — sum for billing
9. API key rotation: support multiple keys, rotate on rate limit, store in secret manager
10. Response validation: check for empty responses, refusals, format compliance before returning
11. Temperature/top_p: expose as user config — different tasks need different creativity levels
12. Model routing: use fast/cheap models for simple tasks, powerful models for complex reasoning
