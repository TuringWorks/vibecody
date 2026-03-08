---
triggers: ["self improvement loop", "lessons learned", "learn from mistakes", "capture lesson", "record correction", "prevent mistake"]
tools_allowed: ["read_file", "write_file"]
category: workflow
---

# Self-Improvement Loop

Continuous learning from corrections and mistakes:

## When to Record a Lesson
- After ANY correction from the user
- When a bug was introduced that could have been prevented
- When a pattern is discovered that should be followed consistently
- When a tool or approach turns out to be superior

## Lesson Format
Each lesson in `tasks/lessons.md` follows:
```
- **#ID** [category]: pattern → rule
```
Example:
```
- **#1** [rust]: Used unwrap() in handler → Always use ? or map_err() in request handlers
- **#2** [testing]: Forgot to test edge case → Always test empty input and boundary values
- **#3** [security]: Hardcoded API key → Use environment variables for all secrets
```

## Categories
- `general` — cross-cutting concerns
- `rust` / `typescript` / `python` — language-specific
- `testing` — test coverage and quality
- `security` — security practices
- `performance` — optimization patterns
- `architecture` — design decisions
- `debugging` — diagnostic techniques

## Feedback Loop
1. User corrects a mistake → record lesson immediately
2. Lesson is injected into future agent system prompts
3. Hit count tracks how often a lesson is relevant
4. Review lessons at session start to prime context
5. Delete or update lessons that are no longer relevant
