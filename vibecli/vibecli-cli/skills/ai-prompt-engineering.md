---
triggers: ["prompt engineering", "system prompt", "few-shot", "chain of thought", "CoT", "structured output", "prompt design"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# AI Prompt Engineering

When designing prompts for LLMs:

1. System prompt: define role, constraints, output format — this is your "programming interface"
2. Be specific: "List 5 security vulnerabilities in this code" beats "Review this code"
3. Few-shot examples: provide 2-3 input/output pairs to demonstrate expected format
4. Chain of Thought (CoT): "Think step by step" improves reasoning on complex tasks
5. Structured output: request JSON with a schema — `"Respond in JSON: {\"issues\": [{\"severity\": ..., \"description\": ...}]}"`
6. Negative instructions work: "Do NOT include explanations" is clear and effective
7. Use delimiters for input sections: triple backticks, XML tags, or markdown headers
8. Temperature: 0.0 for deterministic/factual, 0.7 for creative, 1.0 for brainstorming
9. Max tokens: set based on expected output length — don't waste budget on unused capacity
10. Iterate prompts: test with diverse inputs, track failures, refine instructions
11. Use `<thinking>` or scratchpad sections for complex reasoning before the final answer
12. Avoid prompt injection: validate and sanitize user-provided content before inserting into prompts
