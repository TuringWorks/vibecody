# Discussion / Brainstorm Mode

Pause building to brainstorm with AI about design, architecture, and UX decisions.

## Triggers
- "discussion mode", "brainstorm", "design critique", "let's discuss"
- "pause build", "talk about", "review design", "architecture discussion"

## Usage
```
/discuss start "Auth module redesign"        # Start discussion
/discuss brainstorm                          # Switch to brainstorm mode
/discuss suggest "Use OAuth2 instead"        # Add suggestion
/discuss concern "Migration complexity"      # Raise concern
/discuss decide "Go with OAuth2 + PKCE"      # Record decision
/discuss action "Update schema by Friday"    # Add action item
/discuss summary                             # Get discussion summary
/discuss resume                              # Resume building
/discuss sessions                            # List all discussions
```

## Discussion Modes
- **Brainstorm** — Free-form idea generation
- **Review** — Structured code/design review
- **DesignCritique** — UX and visual design feedback
- **TechDecision** — Technology choice evaluation
- **ArchitectureReview** — System architecture assessment

## Features
- 7 message types: Question, Answer, Suggestion, Concern, Decision, Action, Note
- 5 reaction types: Agree, Disagree, Interesting, NeedsMoreInfo, Resolved
- Build state toggling (Building ↔ Discussing ↔ Paused)
- Decision and action item extraction
- Unresolved concern tracking
- Discussion summary generation
