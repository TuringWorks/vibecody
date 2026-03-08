---
triggers: ["workflow orchestration", "orchestrate", "plan before build", "lessons learned", "self improvement", "demand elegance", "verification gate"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Workflow Orchestration

Structured AI-assisted development workflow with feedback loops:

## 1. Plan Node Default
- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately — don't keep pushing
- Use plan mode for verification steps, not just building
- Write detailed specs upfront to reduce ambiguity

## 2. Subagent Strategy
- Use subagents liberally to keep main context window clean
- Offload research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One task per subagent for focused execution

## 3. Self-Improvement Loop
- After ANY correction from the user: update `tasks/lessons.md` with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review lessons at session start for relevant project

## 4. Verification Before Done
- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness

## 5. Demand Elegance (Balanced)
- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes — don't over-engineer
- Challenge your own work before presenting it

## 6. Autonomous Bug Fixing
- When given a bug report: just fix it. Don't ask for hand-holding
- Point at logs, errors, failing tests — then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how

## Task Management
1. Write plan to `tasks/todo.md` with checkable items
2. Check in before starting implementation
3. Mark items complete as you go
4. Explain changes — high-level summary at each step
5. Add review section to `tasks/todo.md`
6. Capture lessons in `tasks/lessons.md` after corrections

## REPL Commands
- `/orchestrate status` — show orchestration state
- `/orchestrate lessons` — view learned lessons
- `/orchestrate lesson <text>` — record a new lesson
- `/orchestrate todo` — show current task plan
- `/orchestrate todo add <text>` — add a task item
- `/orchestrate todo done <id>` — mark task item complete
- `/orchestrate verify` — run verification gate
- `/orchestrate reset` — clear current task state
