# Skill Distillation — Cross-Session Learning

Automatically extracts coding patterns from your sessions and distills them into reusable skills. Learns your library preferences, naming conventions, error handling patterns, and file organization.

## Pattern Types
- **LibraryPreference** — Preferred libraries (e.g., tokio over async-std)
- **NamingConvention** — snake_case, camelCase, PascalCase preferences
- **FileOrganization** — Test directory structure, component layout
- **ErrorHandling** — Result vs panic, error library preferences
- **TestStyle** — Test naming, assertion style, mock patterns
- **CodeStyle** — Formatting, import ordering, module structure
- **ArchitecturePattern** — Design patterns, layering preferences
- **ConfigPreference** — Config file formats, env var patterns

## Confidence Levels
- **Tentative** — Seen 1-2 times, may be coincidental
- **Confident** — Seen 3+ times, likely intentional
- **Established** — Promoted by user, definitely a preference

## Commands
- `/distill status` — Show learning metrics
- `/distill patterns` — List all learned patterns
- `/distill types` — Show patterns grouped by type
- `/distill export` — Export as skill markdown files
- `/distill reset --confirm` — Clear all learned patterns

## How It Works
1. After each coding session, patterns are extracted from edits
2. Repeated patterns gain confidence over time
3. High-confidence patterns are distilled into skills
4. Skills are injected into future agent prompts

## Example
```
/distill status
/distill patterns
/distill export
```
