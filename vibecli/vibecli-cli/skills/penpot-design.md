---
name: "Penpot Design Integration"
description: "Penpot Design Integration: Open-source Figma alternative. Use when the task involves Penpot, open-source Figma, penpot design."
category: design
triggers: ["Penpot", "open-source Figma", "penpot design"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Penpot Design Integration

Open-source Figma alternative. Self-hosted or cloud (design.penpot.app).

## Setup
1. Get access token: Penpot → Settings → Access Tokens → New Token
2. Configure in DesignMode → Penpot tab → Connect
3. Default cloud URL: `https://design.penpot.app`

## API Reference
All endpoints: `{host}/api/rpc/command/{method}`

| Command | Method | Params |
|---|---|---|
| Get profile | GET | `get-profile` |
| List projects | GET | `get-all-projects` |
| Get files | GET | `get-project-files?project-id={id}` (older instances: `get-files`) |
| Get file | GET | `get-file?id={id}` |
| Export file | POST | `export-binfile` with `file-id, page-id, object-ids, format` |
| Duplicate file | POST | `duplicate-file` with `file-id, name` |

## Tauri Commands
```
connect_penpot(host, token) → { status, host, profile, projects }
list_penpot_files(host, token, projectId) → { files, project_id }
import_penpot_file(host, token, fileId, workspacePath?, provider?) → { file_id, name, components, tokens }
export_penpot_component(host, token, componentId, framework, workspacePath, provider) → String
export_penpot_tokens(tokens, format) → String
```

The first three perform the HTTP request and return an `Err` carrying the
status and Penpot's own error body when it fails. They never echo the access
token back to the caller.

`connect_penpot` calls `get-profile` before `get-all-projects`, so "connected"
means a token that authenticated — an empty project list afterwards is a real
answer rather than an unmade request.

`export_penpot_component` uses the provider selected in the toolbar and errors
when none is; it never falls back to a vendor of its own.

## Token Extraction
Penpot files expose:
- **Colors**: Named hex values (extracted as CSS color tokens)
- **Typographies**: Font family, size, weight (extracted as typography tokens)
- **Components**: Shared component library (reusable across files)

## Component Export Frameworks
- `react` → TypeScript function component with Props interface
- `vue` → Vue 3 SFC with `<template>`, `<script setup>`, `<style scoped>`
- `svelte` → Svelte component with typed exports
- `next.js` → Next.js compatible React component
- `html` → Semantic HTML with BEM CSS

## Self-Hosted Setup
```yaml
# docker-compose.yml excerpt
penpot-frontend:
  image: penpotapp/frontend:latest
  ports: ["9001:80"]
penpot-backend:
  image: penpotapp/backend:latest
  environment:
    PENPOT_PUBLIC_URI: "http://localhost:9001"
```
Then use `http://localhost:9001` as the host.

## Agent Guidance
- Validate config before connecting: check host starts with http(s):// and token non-empty
- Extract tokens AFTER file import — tokens live in `data.colors` and `data.typographies`
- Components are in `data.components` — iterate to build catalogue
- Use `export_penpot_tokens` with format `css` for immediate use in web projects
