# Design System Hub

Cross-provider design token registry with audit, drift detection, and multi-format export.

## Core Concepts
- **Token Namespace**: Named group of tokens from one provider (e.g., "colors", "spacing")
- **Design System**: Collection of namespaces + component catalogue + version
- **Token Drift**: Value changes between two versions of a design system
- **Audit**: Structural checks for completeness and consistency

## Token Types
| Type | ID | Example Values |
|---|---|---|
| Color | `color` | `#3b82f6`, `rgba(59, 130, 246, 0.5)` |
| Typography | `typography` | `Inter, sans-serif`, `16px` |
| Spacing | `spacing` | `4px`, `1rem`, `16px` |
| Border Radius | `border_radius` | `4px`, `50%` |
| Shadow | `shadow` | `0 2px 4px rgba(0,0,0,0.1)` |
| Animation | `animation` | `200ms ease-in-out` |
| Breakpoint | `breakpoint` | `768px`, `1280px` |
| Z-Index | `z_index` | `100`, `9999` |

## Export Formats
### CSS Custom Properties
```css
:root {
  --accent-blue: #3b82f6;
  --space-4: 16px;
}
```

### Tailwind Config
```js
module.exports = {
  theme: { extend: { colors: { "accent-blue": "#3b82f6" } } }
};
```

### TypeScript
```typescript
export const tokens = {
  color: { accent_blue: "#3b82f6" },
} as const;
```

### Style Dictionary (JSON)
```json
{ "colors": { "primary": { "value": "#3b82f6", "type": "color" } } }
```

## Audit Issue Codes
| Code | Severity | Resolution |
|---|---|---|
| `NO_COLORS` | warning | Add color namespace |
| `NO_TYPOGRAPHY` | warning | Add font family/size tokens |
| `NO_SPACING` | info | Add spacing scale (4/8/16/24/32px) |
| `DUPLICATE_TOKENS` | error | Rename tokens to unique names |
| `NO_COMPONENTS` | info | Register components in catalogue |

## Tauri Commands
```
load_design_system_tokens(providers, workspacePath) → { tokens }
export_design_tokens(tokens, format, systemName) → String
audit_design_system_tokens(tokens, systemName) → AuditReport
detect_token_drift(baseline, current) → [TokenDrift]
```

## VibeCody Default Design System
Pre-configured with:
- 15 color tokens (accent-blue, bg-primary, text-primary, etc.)
- 8 spacing tokens (space-1 through space-16)
- 8 typography tokens (font-mono, font-sans, font-size-xs through 2xl)

## Agent Guidance
- Always run `audit_design_system_tokens` after aggregating tokens from multiple providers
- Use `detect_token_drift` when comparing design systems across versions
- Merge tokens from multiple providers using `merge_provider_tokens` with `prefer_provider`
- Export CSS variables for immediate drop-in usage in web projects
- Score < 60: critical issues — fix errors before using in production
- Score 60–79: review warnings — missing token categories
- Score 80+: healthy design system
