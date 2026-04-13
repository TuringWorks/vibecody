# AI Diagram Generator

Generate software architecture, flow, data model, and sequence diagrams from natural language.

## Supported Formats
| Format | ID | Best For |
|---|---|---|
| Mermaid | `mermaid` | Flowcharts, sequence, class, ER, state, Gantt, mind map |
| PlantUML | `plantuml` | UML class, component, sequence, deployment |
| C4 DSL (Structurizr) | `c4` | Architecture context/container/component/code |
| Draw.io XML | `drawio` | Architecture, network topology, ERD, wireframes |

## Diagram Kinds
| Kind | Recommended Format | Use Case |
|---|---|---|
| `flowchart` | Mermaid | Business processes, algorithms, CI/CD pipelines |
| `sequence` | Mermaid | API calls, auth flows, microservice communication |
| `class_diagram` | Mermaid | Domain models, OOP hierarchies |
| `entity_relationship` | Mermaid | Database schemas |
| `c4_context` | C4 DSL | High-level system context |
| `c4_container` | C4 DSL | System internals with technologies |
| `c4_component` | C4 DSL | Component-level design |
| `architecture` | Draw.io | Full system architecture |
| `state_machine` | Mermaid | Lifecycle state flows |
| `mind_map` | Mermaid | Topic decomposition |
| `gantt` | Mermaid | Project timelines |
| `network_topology` | Draw.io | Infrastructure diagrams |

## Tauri Commands
```
generate_diagram(description, kind, format, workspacePath, provider) → String
save_diagram_file(content, filename, workspacePath) → ()
```

## Built-in Templates (Mermaid)
- `MermaidTemplates::microservices_architecture()` — Gateway, services, Kafka, Redis
- `MermaidTemplates::rest_api_sequence()` — Client → Gateway → Auth → Service → DB
- `MermaidTemplates::domain_model()` — User, Order, OrderItem, Product, Payment
- `MermaidTemplates::ci_cd_pipeline()` — Full CI/CD with scan, staging, manual approval
- `MermaidTemplates::er_saas_schema()` — Tenant, User, Project, Task, Comment

## Post-Processing
LLM output is automatically post-processed:
1. Strip markdown fences (```mermaid...```)
2. Validate format-specific root keywords
3. Fix incomplete PlantUML (append `@enduml` if missing)
4. Return structured error if validation fails

## Mermaid Preview
Use the live preview pane (right side) — renders via mermaid.js CDN with dark theme. Available for Mermaid format diagrams only.

## Agent Guidance
- For architecture questions, prefer C4 Context → C4 Container hierarchy
- Use Mermaid for quick prototyping (inline preview available)
- Use draw.io XML when user needs to edit the diagram in the Draw.io editor
- Include system context (team size, tech stack) in the description for better output
- Post-process all LLM output via `post_process_diagram_output` before displaying

## Example Prompts
```
"User registration flow with email verification and rate limiting"
→ kind: flowchart, format: mermaid

"OAuth 2.0 authorization code grant between SPA, backend, and identity provider"
→ kind: sequence, format: mermaid

"E-commerce multi-tenant SaaS with Tenant, User, Product, Order, LineItem, Payment"
→ kind: entity_relationship, format: mermaid

"Online banking system context with retail customers, business customers, back-office staff"
→ kind: c4_context, format: c4
```
