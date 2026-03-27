# Agent Skills Standard

Cross-tool agent skill format for importing, exporting, and validating portable skill definitions. Enables skill sharing between VibeCody, Claude Code, Cursor, Windsurf, and other AI coding tools using a standardized schema.

## When to Use
- Exporting VibeCody skills to share with other agent platforms
- Importing skill packs from community repositories or teammates
- Validating skill files conform to the cross-tool standard schema
- Converting between proprietary skill formats (Cursor rules, Windsurf flows)
- Building a team skill library with version control and approval workflows

## Commands
- `/skills export <name>` — Export a skill to standard JSON format
- `/skills import <path-or-url>` — Import a skill from file or URL
- `/skills validate <path>` — Validate a skill file against the schema
- `/skills convert <format> <path>` — Convert from cursor-rules/windsurf/aider format
- `/skills list --standard` — List all skills with standard compatibility status
- `/skills pack <glob>` — Bundle multiple skills into a distributable pack
- `/skills diff <a> <b>` — Compare two skill versions for changes

## Examples
```
/skills export rust-review
# Exported to ./skills/rust-review.skill.json (v1.2 standard)

/skills import https://hub.example.com/skills/security-audit.json
# Imported: security-audit (3 triggers, 2 tools, validated OK)

/skills convert cursor-rules .cursorrules
# Converted 14 rules to 14 VibeCody skills
```

## Best Practices
- Include clear trigger conditions so skills activate at the right time
- Version your skills using semver for safe updates across teams
- Validate after every edit to catch schema drift early
- Use the pack command for distributing related skills together
- Keep skill descriptions under 200 characters for cross-tool compatibility
