# Design Providers — Multi-Tool Interop

VibeCody supports Figma, Penpot, Pencil (Evolus + TuringWorks), Draw.io, Mermaid, PlantUML, C4 Model, and built-in design capabilities through a unified provider abstraction.

## Provider Quick Reference

| Provider | ID | Capabilities | Auth |
|---|---|---|---|
| Figma | `figma` | Import frames, components, tokens | Personal Access Token |
| Penpot | `penpot` | Full REST API, self-hosted | Access Token (Settings → Access Tokens) |
| Pencil (Evolus) | `pencil` | .ep wireframe parse/generate | None |
| Pencil (TuringWorks) | `pencil_mcp` | .pen file read/write via MCP | None (local MCP server) |
| Draw.io | `drawio` | XML editor, MCP bridge, AI generation | None |
| Mermaid | `mermaid` | AI-generated diagrams, live preview | None |
| PlantUML | `plantuml` | UML diagrams with @startuml | None |
| C4 Model | `c4` | Context/Container/Component/Code | None |
| Built-in | `inhouse` | VibeCody design system tokens | None |

## Token Export Formats
- **CSS**: `:root { --token-name: value; }` — drop into any web project
- **Tailwind**: `theme.extend.colors / spacing` block — paste into tailwind.config.js
- **TypeScript**: `export const tokens = { ... } as const` — type-safe token access
- **Style Dictionary**: JSON format for Amazon Style Dictionary pipeline

## Design System Audit Checks
1. **NO_COLORS** (warning): No color tokens defined
2. **NO_TYPOGRAPHY** (warning): No typography tokens defined
3. **NO_SPACING** (info): No spacing scale tokens
4. **DUPLICATE_TOKENS** (error): Same token name appears in multiple namespaces
5. **NO_COMPONENTS** (info): Component catalogue is empty

## Agent Guidance
- Use `load_design_system_tokens` to aggregate tokens from multiple providers
- Use `export_design_tokens` with format `css | tailwind | typescript | json`
- Use `audit_design_system_tokens` to check for gaps and duplicates
- Token drift detection: compare baseline vs current via `detect_token_drift`
