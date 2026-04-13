# Reasoning Provider

Wrap AI provider calls with reasoning/thinking model support (o3-class, extended thinking). Budgets thinking tokens separately from response tokens.

## When to Use
- Sending prompts to o3, Claude extended-thinking, or any model that emits `<thinking>` blocks
- Controlling how many tokens the model may spend reasoning vs. responding
- Stripping internal chain-of-thought from the user-visible response
- Scaling token budgets proportionally to task complexity (1–10 scale)

## Commands
- `/reasoning budget <complexity>` — Show the token budget for a given complexity level (1–10)
- `/reasoning strip <file>` — Strip `<thinking>` blocks from a raw model output file
- `/reasoning parse <file>` — List all thinking blocks found in a raw output file
- `/reasoning run --tier extended --complexity 8 "<prompt>"` — Run a prompt with extended thinking

## Examples
```
/reasoning budget 7
# max_thinking_tokens: 11093  max_response_tokens: 22938

/reasoning strip output.txt
# Removed 2 thinking blocks (438 tokens). Clean response written to output_clean.txt.

/reasoning run --tier reasoning --complexity 5 "Prove P != NP"
# [thinking stripped]  Final answer: ...
```
