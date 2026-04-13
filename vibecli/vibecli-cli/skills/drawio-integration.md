# Draw.io Integration

Deep integration with draw.io (diagrams.net) for architecture, flowchart, ERD, sequence, and C4 diagrams.

## Key Capabilities
- **Editor**: Full draw.io editor embedded via embed.diagrams.net with postMessage bridge
- **Preview**: Read-only viewer.diagrams.net rendering of any .drawio XML
- **AI Generation**: Natural language → draw.io XML via LLM + `generate_drawio_xml`
- **Templates**: 8 built-in templates (microservices, CI/CD, ERD, C4 context/container, etc.)
- **MCP Bridge**: drawio-mcp commands for file read/write/export
- **Parsing**: Lightweight XML extraction of cells, pages, vertices, edges

## Template Library
| Template ID | Diagram Type | Description |
|---|---|---|
| `microservices` | Architecture | Gateway, services, event bus, cache |
| `ci_cd` | Flowchart | Lint → test → build → deploy pipeline |
| `er_saas` | ERD | Tenant, User, Project, Task, Comment |
| `c4_context` | C4 Context | System boundary with people and external systems |
| `c4_container` | C4 Container | Containers with technologies |
| `api_sequence` | Sequence | REST API with gateway, auth, service, DB |
| `state_order` | State Machine | Order lifecycle states |
| `domain_model` | Class Diagram | User, Order, Product, Payment domain |

## Tauri Commands
```
generate_drawio_xml(description, kind, workspacePath, provider) → String
get_drawio_template(templateId, workspacePath) → String
parse_drawio_xml(xml) → ParsedDrawio
save_drawio_file(xml, workspacePath) → ()
execute_drawio_mcp(command, filePath, content?) → String
export_drawio_svg(filePath, outputPath) → String
```

## MCP Commands (jgraph/drawio-mcp)
```
drawio/read_file { path }
drawio/write_file { path, content }
drawio/list_pages { path }
drawio/get_page { path, page }
drawio/export { path, format: "svg", output }
```

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
