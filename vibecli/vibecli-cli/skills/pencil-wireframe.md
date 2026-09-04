---
name: "Pencil Wireframe Integration"
description: "Pencil Wireframe Integration: Two Pencil integrations: Evolus Pencil (.ep format) for wireframes and TuringWorks Pencil MCP for .pen design files. Use when the task involves Pencil, Evolus Pencil, wireframe, .ep format, .pen design file."
category: design
triggers: ["Pencil", "Evolus Pencil", "wireframe", ".ep format", ".pen design file"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Pencil Wireframe Integration

Two Pencil integrations: Evolus Pencil (.ep format) for wireframes and TuringWorks Pencil MCP for .pen design files.

## Evolus Pencil (.ep Format)
- .ep files are ZIP archives containing `content.xml`
- XML structure: `<Document> → <Page> → <Shape>`
- Shape types: rectangle, ellipse, text, line, arrow, image, button, input, checkbox, radio, dropdown, textarea, table, browser, mobile, container
- Coordinates in pixels, no nesting required

### Parse EP XML
```
parse_pencil_ep(xml: String) → { name, id, pages[], page_count, total_shapes }
```
Errors on empty input and on XML with no `<Document>` element. It reports the
pages the document has — never a floor of 1 — so "no pages" stays "no pages".

### Generate Wireframe Templates
```
generate_pencil_wireframe(templateId, title, sections, workspacePath, provider) → WireframeResult
```

| Template ID | Description |
|---|---|
| `landing_page` | Hero, nav, features, CTA sections |
| `dashboard` | Sidebar navigation, stat cards, chart area |
| `mobile_app` | Status bar, nav, tab bar (multi-screen) |
| `login_form` | Email/password with social auth |
| `settings_page` | Grouped settings with toggles |
| `data_table` | Filterable/sortable data table |

`sections` is the user's comma-separated list. `dashboard` reads it as sidebar
sections, `mobile_app` as one screen per entry, `settings_page` as setting
groups and `data_table` as columns; `landing_page` and `login_form` ignore it.
An empty list falls back to each template's defaults. An unknown template id is
an error, not a substituted wireframe.

### Export a wireframe
```
export_pencil_wireframe(xml, format, workspacePath?, provider?)
  → { filename, mimeType, encoding: "utf8" | "base64", data }
```

| Format | Result |
|---|---|
| `ep` | The `.ep` archive — a ZIP whose `content.xml` is the document. Base64. |
| `ep_xml` | The raw `content.xml`, unzipped. |
| `html` | A standalone page, one absolutely positioned block per shape. Local, deterministic, no provider. |
| `react` | A React component, converted by the **selected** provider. Errors when none is selected. |

Only `react` needs a provider or a network. Writing the raw XML under a `.ep`
name produces a file Pencil cannot open: `.ep` is a ZIP.

## TuringWorks Pencil MCP
The MCP server reads/writes .pen files via standardized tool calls.

### Key Operations
```
get_editor_state({ include_schema })   → Active file + selection state
open_document(path | "new")            → Open or create .pen file
batch_get(patterns, nodeIds)           → Read nodes by pattern/id
batch_design(operations)               → Create/update/delete nodes
get_guidelines(category?)              → Load design guidelines/styles
get_screenshot()                       → Capture current canvas state
get_variables()                        → Read design variable values
set_variables(updates)                 → Update design variables
```

### batch_design Operation Syntax
```
foo=I("parent", { ... })               # Insert new node
baz=C("nodeid", "parent", {...})       # Copy node
foo2=R("nodeid", {...})                # Replace node
U(foo+"/nodeid", {...})                # Update node
D("nodeid")                            # Delete node
```

## Agent Guidance
- Use Evolus Pencil templates for quick wireframe generation and .ep file export
- Every template round-trips through `parse_ep_xml` and is checked for XML
  well-formedness in the tests; keep that true when adding one
- The Pencil MCP bridge is **not dispatched** from VibeCody yet:
  `execute_pencil_mcp` returns the request it would send, labelled
  `status: "not_dispatched"`. Do not report its result as an executed operation
- Use TuringWorks MCP for reading/writing .pen files in the active Pencil editor
- Always call `get_editor_state` before `batch_get` or `batch_design` to confirm active file
- Design tokens extracted from Pencil shapes include fill colors as CSS hex values
