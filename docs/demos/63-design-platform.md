---
layout: page
title: "Demo 63: Multi-Provider Design Platform"
permalink: /demos/63-design-platform/
---

## Overview

VibeCody's Design panel now supports five integrated design and diagramming providers — Figma, **Penpot**, **Evolus Pencil**, **Draw.io**, and an in-house **AI diagram generator** — plus a cross-provider **Design System Hub** for token management. This makes VibeCody the only AI coding assistant with deep design-to-code capabilities built in.

**Time to complete:** ~20 minutes

## Prerequisites

- VibeUI desktop app running (Tauri 2)
- Optional: Penpot account (cloud at `design.penpot.app` or self-hosted)
- Optional: Draw.io Desktop or browser access

---

## Tab 1 — Draw.io Deep Integration

The Draw.io tab embeds a full `diagrams.net` editor via `embed.diagrams.net` with a postMessage bridge for bidirectional communication.

### Sub-tabs

| Sub-tab | What it does |
|---|---|
| **Editor** | Full draw.io editor — create, edit, and save `.drawio` files without leaving VibeUI |
| **Preview** | Read-only viewer for any `.drawio` XML |
| **AI Generate** | Describe a diagram in plain English and get draw.io XML back |
| **Templates** | 8 built-in templates (microservices, CI/CD, ERD, C4 context/container, API sequence, state machine, domain model) |
| **MCP Bridge** | Execute `jgraph/drawio-mcp` commands: read, write, export SVG, list pages |

### AI Diagram Generation

```
Input: "OAuth 2.0 authorization code grant between SPA, backend, and identity provider"
→ kind: sequence
→ format: drawio_xml
→ Output: Complete <mxfile> XML with sequence diagram
```

### C4 Model Templates

The `c4_context` and `c4_container` templates follow the C4 Model conventions:
- **Blue** (`#1168bd`): internal systems and containers
- **Grey** (`#999999`): external systems and actors
- Context → Container → Component hierarchy preserved

---

## Tab 2 — Pencil Wireframes

### Evolus Pencil (.ep format)

The Pencil tab generates low-fidelity wireframes in the Evolus Pencil format — ZIP archives containing `content.xml` with `<Document>/<Page>/<Shape>` structure.

**Available templates:**

| Template | Shapes generated |
|---|---|
| `landing_page` | Hero, navbar, feature cards, CTA, footer |
| `dashboard` | Sidebar, stat cards, chart area, table |
| `mobile_app` | Status bar, nav, content, tab bar (3 screens) |
| `login_form` | Email/password, social auth buttons |
| `settings_page` | Grouped settings with toggles |
| `data_table` | Filter bar, column headers, rows |

### TuringWorks Pencil MCP

For `.pen` files in the active Pencil editor, use the MCP tab to send `batch_design` operations:

```
foo=I("parent", { type: "rectangle", x: 100, y: 100, width: 200, height: 60 })
U(foo, { fill: "#3b82f6" })
```

---

## Tab 3 — Penpot Integration

[Penpot](https://penpot.app) is the open-source Figma alternative. VibeCody connects via the REST API at `{host}/api/rpc/command/{method}`.

### Setup

1. Open VibeUI → Design → Penpot → Connect tab
2. Enter your Penpot host (e.g. `https://design.penpot.app`) and access token
3. Click **Connect** — projects and files load automatically

### Token Export

Colors and typographies are extracted from the Penpot file's design data:

```css
/* CSS custom properties */
:root {
  --accent-blue: #3b82f6;
  --font-heading: Inter, sans-serif;
}
```

### Component Export

Select any component and export to your target framework:

| Framework | Output |
|---|---|
| `react` | TypeScript function component with Props interface |
| `vue` | Vue 3 SFC `<template>/<script setup>/<style scoped>` |
| `svelte` | Svelte component with typed exports |
| `next.js` | Next.js-compatible React component |
| `html` | Semantic HTML + BEM CSS |

---

## Tab 4 — AI Diagram Generator

The Diagrams tab generates architecture, flow, ER, sequence, and other diagrams from plain-English descriptions using any configured AI provider.

### Supported Output Formats

| Format | Best for |
|---|---|
| `mermaid` | Quick flowcharts, sequences, class diagrams, ER — live preview available |
| `plantuml` | UML class, component, deployment diagrams |
| `c4_dsl` | Structurizr DSL for C4 architecture models |
| `drawio_xml` | Full draw.io diagrams editable in Desktop |

### Live Mermaid Preview

Mermaid diagrams render immediately in the right pane via `mermaid.js` (dark theme):

```
Input: "User registration flow with email verification and rate limiting"
→ kind: flowchart
→ format: mermaid
→ Live preview: rendered flowchart with swim lanes
```

### Sample Prompts (click to use)

1. User registration flow with email verification
2. OAuth 2.0 authorization code grant
3. Microservices architecture with gateway and event bus
4. E-commerce ER diagram: User, Product, Order, Payment
5. CI/CD pipeline with scan, staging, manual approval
6. SaaS tenant isolation context diagram (C4)

---

## Tab 5 — Design System Hub

The Hub aggregates tokens from all connected providers into a unified browser.

### Token Types

| Type | Example |
|---|---|
| Color | `#3b82f6`, `rgba(59,130,246,0.5)` |
| Typography | `Inter, sans-serif`, `16px` |
| Spacing | `4px`, `1rem`, `16px` |
| Border Radius | `4px`, `50%` |
| Shadow | `0 2px 4px rgba(0,0,0,0.1)` |
| Animation | `200ms ease-in-out` |

### Export Formats

```css
/* CSS Custom Properties */
:root { --accent-blue: #3b82f6; --space-4: 16px; }
```

```js
// Tailwind Config
module.exports = { theme: { extend: { colors: { "accent-blue": "#3b82f6" } } } };
```

```typescript
// TypeScript const
export const tokens = { color: { accent_blue: "#3b82f6" } } as const;
```

```json
// Style Dictionary JSON
{ "colors": { "primary": { "value": "#3b82f6", "type": "color" } } }
```

### Audit

The audit tab scores your design system 0–100 and flags structural issues:

| Code | Severity | Issue |
|---|---|---|
| `NO_COLORS` | warning | No color tokens defined |
| `NO_TYPOGRAPHY` | warning | No font tokens |
| `NO_SPACING` | info | No spacing scale |
| `DUPLICATE_TOKENS` | error | Token names not unique |
| `NO_COMPONENTS` | info | Empty component catalogue |

**Score interpretation:** ≥80 healthy · 60–79 review warnings · <60 critical issues

### Token Drift

Compare two snapshots of a design system to see what changed:

```
Drift detected:
  accent-blue: #3b82f6 → #2563eb  (changed)
  space-8: 32px → 8px             (changed)
  font-mono: added
```

---

## VibeCody Default Design System

When no external provider is connected, VibeCody loads its built-in design system:
- **15 color tokens**: accent-blue, bg-primary, bg-secondary, text-primary, text-secondary, border-color, success, warning, error, info, purple, green, orange, surface, overlay
- **8 spacing tokens**: space-1 (4px) through space-16 (64px)
- **8 typography tokens**: font-mono, font-sans, font-size-xs through font-size-2xl

---

## Related Demos

- [Demo 64: Reasoning Provider & Extended Thinking](../64-reasoning-provider/)
- [Fit-Gap Analysis](../../fit-gap-analysis/) — consolidated competitive catalogue
