# Enterprise Architecture Specification

Unified architecture framework supporting TOGAF ADM, Zachman Framework, C4 Model, and Architecture Decision Records (ADRs). Includes governance engine for compliance validation.

## Frameworks

### TOGAF ADM (Architecture Development Method)
- 9 phases: Preliminary, Architecture Vision, Business Architecture, Information Systems, Technology Architecture, Opportunities & Solutions, Migration Planning, Implementation Governance, Architecture Change Management
- Artifact tracking: Catalogs, Matrices, Diagrams per phase
- Phase prerequisite validation
- Completion tracking and reporting

### Zachman Framework
- 6x6 matrix: Perspectives (Planner/Owner/Designer/Builder/Implementer/Worker) x Aspects (What/How/Where/Who/When/Why)
- Cell maturity scoring (0-5)
- Coverage analysis and gap identification
- Cross-cell consistency validation

### C4 Model
- 4 levels: Context, Container, Component, Code
- Element types: Person, SoftwareSystem, Container, Component
- Mermaid/PlantUML diagram generation
- Model validation (orphan elements, missing relationships)

### Architecture Decision Records (ADRs)
- Full lifecycle: Proposed, Accepted, Deprecated, Superseded
- Markdown export in standard ADR format
- Search and indexing

### Governance Engine
- Rule-based architecture governance
- Violation detection with severity and recommendations
- Cross-framework evaluation

## Commands
- `/archspec togaf <phase>` — Show TOGAF phase artifacts
- `/archspec zachman` — Display Zachman matrix
- `/archspec c4 context|container|component` — Generate C4 diagrams
- `/archspec adr add|list|accept|deprecate` — Manage ADRs
- `/archspec governance check` — Run governance rules
- `/archspec report` — Full architecture report

## Example
```
/archspec togaf vision
/archspec c4 context
/archspec adr add "Use PostgreSQL for primary database"
/archspec zachman
```
