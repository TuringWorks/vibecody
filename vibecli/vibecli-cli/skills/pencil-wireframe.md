# Pencil Wireframe Integration

Two Pencil integrations: Evolus Pencil (.ep format) for wireframes and TuringWorks Pencil MCP for .pen design files.

## Evolus Pencil (.ep Format)
- .ep files are ZIP archives containing `content.xml`
- XML structure: `<Document> → <Page> → <Shape>`
- Shape types: rectangle, ellipse, text, line, arrow, image, button, input, checkbox, radio, dropdown, textarea, table, browser, mobile, container
- Coordinates in pixels, no nesting required

### Parse EP XML
```
parse_pencil_ep(xml: String) → ParseResult
```

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
- Use TuringWorks MCP for reading/writing .pen files in the active Pencil editor
- Always call `get_editor_state` before `batch_get` or `batch_design` to confirm active file
- Design tokens extracted from Pencil shapes include fill colors as CSS hex values
