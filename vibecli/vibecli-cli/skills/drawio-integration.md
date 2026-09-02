---
name: "Draw.io Integration"
description: "Draw.io Integration: Deep integration with draw.io (diagrams.net) for architecture, flowchart, ERD, sequence, and C4 diagrams. Use when the task involves draw.io, diagrams.net, drawio, flowchart, ERD diagram."
category: design
triggers: ["draw.io", "diagrams.net", "drawio", "flowchart", "ERD diagram"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Draw.io Integration

Deep integration with draw.io (diagrams.net) for architecture, flowchart, ERD, sequence, and C4 diagrams.

## Key Capabilities
- **Diagrams**: the workspace's own `.drawio` / `.dio` / `.drawio.svg` files, listed
  newest-first, opened into the editor. This is the entry point — the panel is a
  file editor, not a scratch canvas.
- **Editor**: full draw.io editor embedded via embed.diagrams.net with a
  postMessage bridge. `autosave=1`, so Save writes what is on screen.
- **Preview**: read-only viewer.diagrams.net rendering, diagram delivered by
  postMessage rather than in the URL.
- **AI Generation**: natural language → draw.io XML via LLM + `generate_drawio_xml`
- **Templates**: 8 built-in starter diagrams, served from the backend so the tab
  cannot advertise one this build does not have
- **Export**: PNG / SVG / editable SVG / PDF, written **into the workspace**
  beside the diagram
- **MCP Bridge**: file read/write/inspect by workspace-relative path
- **Parsing**: lightweight XML extraction of cells, pages, vertices, edges

## Where files go

Every path is **workspace-relative** and resolved inside the open workspace;
nothing here writes outside it.

| Action | Destination |
|---|---|
| Save on a diagram opened from the list | back to that file |
| Save on a new diagram | asks for a name; a bare name becomes `diagrams/<name>.drawio` |
| Export | the diagram's own path with the format's extension — `docs/arch.drawio` → `docs/arch.png` |

Saving back into a `.drawio.svg` is **refused by name**: writing only the
embedded XML would leave the rendered picture showing the previous version.
Export an SVG beside the `.drawio` instead.

## Template Library

Defined in `vibecoder/src-tauri/src/drawio_templates.rs`; `list_drawio_templates`
serves the same list the diagrams come from, so the two cannot disagree. Each is
a real starter diagram — a test asserts every template has ≥4 shapes and ≥3
connections, because all eight used to return one rectangle containing their own
name.

| Template ID | Diagram Type | Description |
|---|---|---|
| `microservices` | Architecture | Gateway, three services, their datastores and a message bus |
| `ci_cd` | Flowchart | Commit through build, test, and gated deploy to production |
| `er_saas` | ERD | Accounts, users, subscriptions, plans and invoices |
| `c4_context` | C4 Context | One system, its people, and the systems it talks to |
| `c4_container` | C4 Container | Web app, API, worker and database inside one boundary |
| `api_sequence` | Sequence | Client → gateway → service → database, with the return path |
| `state_order` | State Machine | Placed through delivered, with cancel and refund branches |
| `domain_model` | Class Diagram | Customer, Order, OrderLine and Product with multiplicities |

## Tauri Commands
```
list_drawio_files(workspacePath)                     → DrawioFile[]
read_drawio_file(workspacePath, relativePath)        → String   (XML; extracts it from a .drawio.svg)
save_drawio_file(workspacePath, relativePath, xml)   → DrawioSaved { path, absolute_path, size_bytes, created }
export_drawio_file(workspacePath, relativePath, dataUrl) → DrawioSaved
list_drawio_templates()                              → DrawioTemplate[]
get_drawio_template(templateId)                      → String
generate_drawio_xml(description, kind, workspacePath, provider) → String
parse_drawio_xml(xml)                                → ParsedDrawio
execute_drawio_mcp(command, filePath, workspacePath, content?) → String
```

`DrawioFile.pages` / `.vertices` / `.edges` are **null** for a file too large to
count during the listing, and `.modified_unix` is null where the filesystem
reported no mtime — absent rather than zero, so a large diagram is not described
as an empty one and a file is not stamped "just now" on no evidence.

## Embed parameters, and the button that did not work

The editor URL is
`embed=1&proto=json&configure=1&autosave=1&noExitBtn=1&saveAndExit=0`.

- `configure=1` makes draw.io **block** until the host answers a `configure`
  message. No reply, no `init`; no `init`, blank canvas with no error.
- `autosave=1` makes the editor push every change back. Without it the `autosave`
  handler never runs and Save writes whatever the editor last volunteered.
- `noSaveBtn=1` is **not** set: it replaces *Save* with *Save & Exit*, which
  saved and then did nothing, because nothing on the host handled `exit`.
  The `exit` event is now handled anyway (it closes the document) so the control
  is honest wherever it appears.

## MCP Commands (jgraph/drawio-mcp)
```
drawio/read_file { path }
drawio/write_file { path, content }
drawio/list_pages { path }
drawio/get_page { path, page }
drawio/export { path, format: "svg", output }
```

Not every one of these is wired: `execute_drawio_mcp` implements `read_file`,
`write_file` and `list_pages`, and answers anything else with
`{"status":"queued"}`. Treat that as "not implemented", not as work in progress.

## XML Structure
```xml
<mxfile>
  <diagram name="Page-1" id="...">
    <mxGraphModel>
      <root>
        <mxCell id="0" />
        <mxCell id="1" parent="0" />
        <mxCell id="2" value="Node" style="rounded=1;" vertex="1" parent="1">
          <mxGeometry x="100" y="100" width="120" height="40" as="geometry" />
        </mxCell>
      </root>
    </mxGraphModel>
  </diagram>
</mxfile>
```

## C4 Model Guidance
- **C4 Context**: Highest level — show system + people + external systems
- **C4 Container**: Expand system boundary to show containers (web app, API, DB)
- **C4 Component**: Expand a container to show its internal components
- Styles: internal = blue (`#1168bd`), external = grey (`#999999`)
