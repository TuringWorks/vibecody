---
triggers: ["Mermaid", "PlantUML", "C4 model", "sequence diagram", "architecture diagram", "flowchart"]
tools_allowed: ["read_file", "write_file", "bash"]
category: documentation
---

# Technical Diagrams

When creating technical diagrams:

1. Use Mermaid for inline markdown diagrams — renders in GitHub, GitLab, Notion, and most tools
2. Sequence diagrams for API flows: show request/response between services with timing
3. Flowcharts for decision logic: `graph TD; A-->B; B{condition}-->|yes|C; B-->|no|D`
4. C4 model for architecture: Context → Container → Component → Code (zoom levels)
5. Entity-Relationship diagrams for database schemas: tables, relationships, cardinality
6. State diagrams for stateful objects: transitions, guards, actions
7. Class diagrams for object models: inheritance, composition, interfaces
8. Use PlantUML for complex diagrams that exceed Mermaid's capabilities
9. Keep diagrams close to code: `docs/diagrams/` or inline in markdown files
10. Label everything: boxes, arrows, and swimlanes should have descriptive text
11. Gantt charts for project timelines: `gantt; section Phase 1; Task A :a1, 2024-01-01, 30d`
12. Update diagrams when architecture changes — automated rendering in CI prevents drift
