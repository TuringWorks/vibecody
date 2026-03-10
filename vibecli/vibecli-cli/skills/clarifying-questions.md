# Plan Mode with Clarifying Questions

Ask clarifying questions before generating implementation plans, ensuring alignment before coding.

## Triggers
- "clarifying questions", "megaplan", "plan mode", "ask before coding"
- "clarify scope", "implementation plan", "plan with questions"

## Usage
```
/clarify "Build a REST API for user management"  # Start session
/clarify answer q-1 "REST with JWT auth"         # Answer question
/clarify skip q-3                                 # Skip with default
/clarify plan                                     # Generate plan (after answers)
/clarify unanswered                               # Show remaining questions
/clarify summary                                  # Show plan summary
```

## Question Categories
- **Scope** — What's included/excluded
- **Architecture** — REST vs GraphQL, monolith vs microservice
- **Dependencies** — External libraries, existing code to reuse
- **Testing** — Unit vs integration, coverage targets
- **Performance** — Expected load, caching needs
- **Security** — Auth strategy, input validation
- **Deployment** — Target environment, CI/CD needs
- **Style** — Code conventions, naming patterns
- **Compatibility** — Backward compatibility, migration needs

## Features
- Automatic question generation from task keyword analysis
- Context-aware questions (API tasks → auth strategy, DB tasks → SQL vs NoSQL)
- MegaPlan generation with steps, file changes, effort estimates
- Risk level assessment (Low/Medium/High/Critical)
- Default answers for skipped questions
- Session lifecycle: Questioning → Answered → PlanReady → Executing
