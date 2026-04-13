//! AI-powered diagram generation — Mermaid, PlantUML, C4 DSL, draw.io XML.
//!
//! Given a natural-language description, generates diagrams in the requested format.
//! Also provides prompt templates for each diagram type.

use serde::{Deserialize, Serialize};

use crate::design_providers::{DiagramDoc, DiagramFormat, DiagramKind, ProviderKind};

// ─── Generation request ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramRequest {
    pub description: String,
    pub kind: DiagramKind,
    pub format: DiagramFormat,
    pub context: Option<String>,
    pub style_hints: Vec<String>,
}

impl DiagramRequest {
    pub fn new(description: &str, kind: DiagramKind) -> Self {
        let format = kind.preferred_format();
        Self {
            description: description.to_string(),
            kind,
            format,
            context: None,
            style_hints: Vec::new(),
        }
    }

    pub fn with_format(mut self, format: DiagramFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_context(mut self, ctx: &str) -> Self {
        self.context = Some(ctx.to_string());
        self
    }
}

// ─── System prompt builders ───────────────────────────────────────────────────

/// Build a system prompt for diagram generation
pub fn build_system_prompt(kind: &DiagramKind, format: &DiagramFormat) -> String {
    let format_instructions = match format {
        DiagramFormat::MermaidMd => "Output ONLY valid Mermaid diagram code. No markdown code fences. No explanations. Start directly with the diagram type keyword (flowchart, sequenceDiagram, classDiagram, etc.).",
        DiagramFormat::DrawIoXml => "Output ONLY valid draw.io XML in mxGraphModel format. Start with <mxGraphModel. No explanations.",
        DiagramFormat::PlantUml => "Output ONLY valid PlantUML code. Start with @startuml and end with @enduml. No explanations.",
        DiagramFormat::C4Dsl => "Output ONLY valid Structurizr DSL (C4 model). Start with workspace { and end with }. No explanations.",
        DiagramFormat::SvgMarkup => "Output ONLY valid SVG markup. Start with <svg and end with </svg>. No explanations.",
        _ => "Output the diagram in the requested format. No explanations.",
    };

    let kind_guidance = match kind {
        DiagramKind::Flowchart => "Create a clear, left-to-right or top-down flowchart with decision diamonds for conditions and rounded rectangles for processes.",
        DiagramKind::Sequence => "Create a sequence diagram showing the temporal ordering of messages between actors/components.",
        DiagramKind::ClassDiagram => "Create a UML class diagram with attributes, methods, inheritance (--|>), composition (*--), and aggregation (o--) relationships.",
        DiagramKind::EntityRelationship => "Create an ER diagram with entities, attributes, and cardinality relationships (||--||, ||--o{, }o--o{).",
        DiagramKind::ComponentDiagram => "Create a UML component diagram showing architectural components and their interfaces/dependencies.",
        DiagramKind::DeploymentDiagram => "Create a deployment diagram showing infrastructure nodes, artifacts, and communication paths.",
        DiagramKind::C4Context => "Create a C4 Context diagram. Show the system boundary, external users (Person), external systems, and high-level relationships.",
        DiagramKind::C4Container => "Create a C4 Container diagram. Expand the system to show containers (web apps, APIs, databases) with technologies.",
        DiagramKind::C4Component => "Create a C4 Component diagram. Expand a container to show its internal components and their interactions.",
        DiagramKind::C4Code => "Create a C4 Code diagram using class or sequence format to show the code-level design.",
        DiagramKind::Architecture => "Create a layered architecture diagram showing all major system components, layers, and their relationships.",
        DiagramKind::StateMachine => "Create a state machine/state diagram with initial state, states, transitions, and events.",
        DiagramKind::MindMap => "Create a mind map with a central topic and radiating subtopics.",
        DiagramKind::Gantt => "Create a Gantt chart with tasks, durations, and dependencies.",
        DiagramKind::UserJourney => "Create a user journey diagram showing a user's experience across different stages.",
        DiagramKind::Wireframe => "Create a wireframe-style diagram showing UI layout with boxes and labels.",
        DiagramKind::NetworkTopology => "Create a network topology diagram showing servers, network devices, and connections.",
    };

    format!("You are an expert software architect and diagram generator.\n\n{}\n\n{}", kind_guidance, format_instructions)
}

/// Build the user prompt for diagram generation
pub fn build_user_prompt(req: &DiagramRequest) -> String {
    let mut prompt = format!("Generate a {} diagram for:\n\n{}", req.kind.display_name(), req.description);
    if let Some(ctx) = &req.context {
        prompt.push_str(&format!("\n\nAdditional context:\n{}", ctx));
    }
    if !req.style_hints.is_empty() {
        prompt.push_str(&format!("\n\nStyle preferences:\n- {}", req.style_hints.join("\n- ")));
    }
    prompt
}

// ─── Post-processing ──────────────────────────────────────────────────────────

/// Clean and validate LLM output for a given diagram format
pub fn post_process_diagram_output(raw: &str, format: &DiagramFormat) -> Result<String, String> {
    let cleaned = raw.trim();
    let cleaned = strip_markdown_fences(cleaned);

    match format {
        DiagramFormat::MermaidMd => validate_mermaid(cleaned),
        DiagramFormat::DrawIoXml => validate_drawio_xml(cleaned),
        DiagramFormat::PlantUml => validate_plantuml(cleaned),
        DiagramFormat::C4Dsl => validate_c4_dsl(cleaned),
        DiagramFormat::SvgMarkup => validate_svg(cleaned),
        _ => Ok(cleaned.to_string()),
    }
}

fn strip_markdown_fences(s: &str) -> &str {
    let s = s.trim_start_matches("```mermaid").trim_start_matches("```drawio")
        .trim_start_matches("```plantuml").trim_start_matches("```c4")
        .trim_start_matches("```xml").trim_start_matches("```svg")
        .trim_start_matches("```");
    let s = if s.ends_with("```") { &s[..s.len() - 3] } else { s };
    s.trim()
}

fn validate_mermaid(s: &str) -> Result<String, String> {
    let valid_keywords = [
        "flowchart", "graph", "sequenceDiagram", "classDiagram", "erDiagram",
        "stateDiagram", "gantt", "pie", "journey", "mindmap", "timeline",
        "gitGraph", "C4Context", "C4Container", "C4Component",
    ];
    let first_word = s.split_whitespace().next().unwrap_or("");
    if valid_keywords.iter().any(|kw| first_word.eq_ignore_ascii_case(kw)) {
        Ok(s.to_string())
    } else {
        // Try to find a valid keyword in the first few lines
        for line in s.lines().take(5) {
            let word = line.split_whitespace().next().unwrap_or("");
            if valid_keywords.iter().any(|kw| word.eq_ignore_ascii_case(kw)) {
                return Ok(s.to_string());
            }
        }
        Err(format!("Invalid Mermaid: no recognized diagram type keyword found. Got: {}", &s[..s.len().min(50)]))
    }
}

fn validate_drawio_xml(s: &str) -> Result<String, String> {
    if s.contains("<mxGraphModel") || s.contains("<mxfile") || s.contains("<?xml") {
        Ok(s.to_string())
    } else {
        Err("Invalid draw.io XML: missing <mxGraphModel or <mxfile root".to_string())
    }
}

fn validate_plantuml(s: &str) -> Result<String, String> {
    if s.contains("@startuml") {
        let end = if !s.contains("@enduml") { format!("{}\n@enduml", s) } else { s.to_string() };
        Ok(end)
    } else {
        Err("Invalid PlantUML: missing @startuml".to_string())
    }
}

fn validate_c4_dsl(s: &str) -> Result<String, String> {
    if s.contains("workspace") || s.contains("softwareSystem") || s.contains("container") || s.contains("person") {
        Ok(s.to_string())
    } else {
        Err("Invalid C4 DSL: missing workspace or structural elements".to_string())
    }
}

fn validate_svg(s: &str) -> Result<String, String> {
    if s.contains("<svg") {
        Ok(s.to_string())
    } else {
        Err("Invalid SVG: missing <svg root element".to_string())
    }
}

// ─── Canned diagram templates for common use cases ───────────────────────────

/// Pre-built Mermaid templates for common software patterns
pub struct MermaidTemplates;

impl MermaidTemplates {
    pub fn microservices_architecture() -> &'static str {
        r#"flowchart TD
    Client[Web/Mobile Client]
    Gateway[API Gateway\nNginx / Kong]
    Auth[Auth Service\nJWT]
    Users[Users Service\nPostgres]
    Orders[Orders Service\nMongo]
    Payments[Payments Service\nStripe]
    Events[Event Bus\nKafka]
    Cache[Cache\nRedis]

    Client --> Gateway
    Gateway --> Auth
    Gateway --> Users
    Gateway --> Orders
    Gateway --> Payments
    Auth --> Cache
    Users --> Events
    Orders --> Events
    Payments --> Events

    style Gateway fill:#4299e1,color:#fff
    style Events fill:#ed8936,color:#fff
    style Cache fill:#48bb78,color:#fff"#
    }

    pub fn rest_api_sequence() -> &'static str {
        r#"sequenceDiagram
    participant C as Client
    participant G as API Gateway
    participant A as Auth Service
    participant S as Business Service
    participant D as Database

    C->>G: POST /api/resource
    G->>A: Validate JWT token
    A-->>G: Token valid + claims
    G->>S: Forward request + claims
    S->>D: Query/Mutate data
    D-->>S: Result
    S-->>G: Response payload
    G-->>C: 200 OK + JSON

    Note over A,S: All inter-service calls use mTLS"#
    }

    pub fn domain_model() -> &'static str {
        r#"classDiagram
    class User {
        +UUID id
        +String email
        +String name
        +DateTime createdAt
        +login() bool
        +logout() void
    }
    class Order {
        +UUID id
        +OrderStatus status
        +Decimal total
        +DateTime placedAt
        +calculate() Decimal
        +confirm() void
    }
    class OrderItem {
        +UUID id
        +int quantity
        +Decimal unitPrice
        +subtotal() Decimal
    }
    class Product {
        +UUID id
        +String name
        +Decimal price
        +int stockLevel
        +reserve(qty) bool
    }
    class Payment {
        +UUID id
        +PaymentMethod method
        +Decimal amount
        +PaymentStatus status
        +process() bool
    }

    User "1" --> "*" Order : places
    Order "1" *-- "*" OrderItem : contains
    OrderItem "*" --> "1" Product : references
    Order "1" --> "0..1" Payment : has"#
    }

    pub fn ci_cd_pipeline() -> &'static str {
        r#"flowchart LR
    Push[Git Push] --> Trigger[CI Trigger]
    Trigger --> Lint[Lint & Format]
    Lint --> Test[Unit Tests]
    Test --> Build[Build Artifacts]
    Build --> Scan[Security Scan\nSonarQube / Snyk]
    Scan --> |Pass| Stage[Deploy to Staging]
    Scan --> |Fail| Notify[Notify Team]
    Stage --> IntTest[Integration Tests]
    IntTest --> |Pass| Approve{Manual Approval}
    IntTest --> |Fail| Rollback[Rollback Stage]
    Approve --> |Approved| Prod[Deploy Production]
    Approve --> |Rejected| Hold[Hold for Review]
    Prod --> Health[Health Checks]
    Health --> |OK| Done[✓ Release Done]
    Health --> |Fail| RollProd[Rollback Prod]

    style Done fill:#48bb78,color:#fff
    style Notify fill:#fc8181,color:#fff
    style Rollback fill:#fc8181,color:#fff"#
    }

    pub fn er_saas_schema() -> &'static str {
        r#"erDiagram
    TENANT {
        uuid id PK
        string name
        string plan
        datetime created_at
    }
    USER {
        uuid id PK
        uuid tenant_id FK
        string email
        string role
        boolean active
    }
    PROJECT {
        uuid id PK
        uuid tenant_id FK
        string name
        string status
        datetime deadline
    }
    TASK {
        uuid id PK
        uuid project_id FK
        uuid assignee_id FK
        string title
        string status
        int priority
    }
    COMMENT {
        uuid id PK
        uuid task_id FK
        uuid author_id FK
        text body
        datetime created_at
    }

    TENANT ||--o{ USER : "has"
    TENANT ||--o{ PROJECT : "owns"
    PROJECT ||--o{ TASK : "contains"
    USER ||--o{ TASK : "assigned to"
    TASK ||--o{ COMMENT : "has""#
    }

    pub fn state_machine_order() -> &'static str {
        r#"stateDiagram-v2
    [*] --> Draft
    Draft --> Submitted : submit()
    Submitted --> UnderReview : assign_reviewer()
    Submitted --> Cancelled : cancel()
    UnderReview --> Approved : approve()
    UnderReview --> Rejected : reject()
    UnderReview --> Submitted : request_changes()
    Approved --> Processing : start_processing()
    Processing --> Shipped : ship()
    Shipped --> Delivered : deliver()
    Delivered --> [*]
    Rejected --> [*]
    Cancelled --> [*]

    note right of Processing
        Payment captured at
        this stage
    end note"#
    }
}

// ─── C4 DSL templates ─────────────────────────────────────────────────────────

pub struct C4Templates;

impl C4Templates {
    pub fn saas_context(system_name: &str) -> String {
        format!(
            r#"workspace "{} Architecture" "C4 Context Diagram" {{
  model {{
    user = person "End User" "Interacts with the system via web or mobile"
    adminUser = person "Admin User" "Manages the platform configuration"
    externalSvc = softwareSystem "External Services" "Third-party APIs (Stripe, SendGrid, etc.)" external

    {} = softwareSystem "{}" "Core SaaS platform" {{
      webapp = container "Web Application" "React SPA" "TypeScript, React"
      api = container "REST API" "Business logic and data access" "Rust / Node.js"
      db = container "Database" "Stores all platform data" "PostgreSQL"
      cache = container "Cache" "Session and query cache" "Redis"
    }}

    user -> {} "Uses"
    adminUser -> {} "Manages"
    {} -> externalSvc "Calls"
  }}

  views {{
    systemContext {} "SystemContext" {{
      include *
      autolayout lr
    }}
    theme default
  }}
}}"#,
            system_name,
            system_name.to_lowercase().replace(' ', "_"),
            system_name,
            system_name.to_lowercase().replace(' ', "_"),
            system_name.to_lowercase().replace(' ', "_"),
            system_name.to_lowercase().replace(' ', "_"),
            system_name.to_lowercase().replace(' ', "_"),
        )
    }
}

// ─── PlantUML templates ───────────────────────────────────────────────────────

pub struct PlantUmlTemplates;

impl PlantUmlTemplates {
    pub fn component_diagram(system_name: &str, components: &[(&str, &str)]) -> String {
        let comps: String = components.iter()
            .map(|(name, tech)| format!("  [{}] <<{}>>", name, tech))
            .collect::<Vec<_>>().join("\n");

        format!(
            r#"@startuml {}
!include https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4_Component.puml

title {} Components

package "{}" {{
{}
}}

@enduml"#,
            system_name.to_lowercase().replace(' ', "_"),
            system_name,
            system_name,
            comps
        )
    }

    pub fn sequence_auth_flow() -> &'static str {
        r#"@startuml auth_flow
actor User
participant "Browser" as B
participant "API Gateway" as G
participant "Auth Service" as A
participant "Resource Service" as R
database "Token Store" as T

User -> B: Enter credentials
B -> G: POST /auth/login
G -> A: Validate credentials
A -> T: Store refresh token
A --> G: Access token + Refresh token
G --> B: 200 OK {tokens}
B -> B: Store tokens (httpOnly cookie)

...Later...

B -> G: GET /api/resource (Bearer token)
G -> A: Verify JWT
A --> G: Valid + claims
G -> R: Forward + user context
R --> G: Resource data
G --> B: 200 OK {data}

@enduml"#
    }
}

// ─── DiagramDoc factory ───────────────────────────────────────────────────────

pub fn make_mermaid_doc(title: &str, kind: DiagramKind, mermaid: &str) -> DiagramDoc {
    DiagramDoc::new(title, kind, mermaid.to_string(), ProviderKind::Mermaid)
}

pub fn make_plantuml_doc(title: &str, kind: DiagramKind, puml: &str) -> DiagramDoc {
    let mut doc = DiagramDoc::new(title, kind, puml.to_string(), ProviderKind::PlantUml);
    doc.format = DiagramFormat::PlantUml;
    doc
}

pub fn make_c4_doc(title: &str, kind: DiagramKind, dsl: &str) -> DiagramDoc {
    let mut doc = DiagramDoc::new(title, kind, dsl.to_string(), ProviderKind::C4Model);
    doc.format = DiagramFormat::C4Dsl;
    doc
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_system_prompt_mermaid_has_keywords() {
        let prompt = build_system_prompt(&DiagramKind::Flowchart, &DiagramFormat::MermaidMd);
        assert!(prompt.contains("Mermaid"));
        assert!(prompt.contains("flowchart"));
    }

    #[test]
    fn build_user_prompt_includes_description() {
        let req = DiagramRequest::new("Order processing system", DiagramKind::Sequence);
        let prompt = build_user_prompt(&req);
        assert!(prompt.contains("Order processing system"));
        assert!(prompt.contains("Sequence"));
    }

    #[test]
    fn post_process_valid_mermaid() {
        let raw = "flowchart TD\n  A-->B";
        let result = post_process_diagram_output(raw, &DiagramFormat::MermaidMd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), raw);
    }

    #[test]
    fn post_process_invalid_mermaid_fails() {
        let raw = "this is not a diagram";
        let result = post_process_diagram_output(raw, &DiagramFormat::MermaidMd);
        assert!(result.is_err());
    }

    #[test]
    fn post_process_strips_fences() {
        let raw = "```mermaid\nflowchart TD\n  A-->B\n```";
        let result = post_process_diagram_output(raw, &DiagramFormat::MermaidMd);
        assert!(result.is_ok());
        assert!(!result.unwrap().contains("```"));
    }

    #[test]
    fn post_process_valid_drawio_xml() {
        let raw = "<mxGraphModel><root></root></mxGraphModel>";
        let result = post_process_diagram_output(raw, &DiagramFormat::DrawIoXml);
        assert!(result.is_ok());
    }

    #[test]
    fn post_process_valid_plantuml() {
        let raw = "@startuml\nA -> B\n@enduml";
        let result = post_process_diagram_output(raw, &DiagramFormat::PlantUml);
        assert!(result.is_ok());
    }

    #[test]
    fn post_process_plantuml_adds_enduml() {
        let raw = "@startuml\nA -> B";
        let result = post_process_diagram_output(raw, &DiagramFormat::PlantUml).unwrap();
        assert!(result.contains("@enduml"));
    }

    #[test]
    fn mermaid_template_microservices_valid() {
        let template = MermaidTemplates::microservices_architecture();
        assert!(template.contains("flowchart"));
        assert!(template.contains("Gateway"));
    }

    #[test]
    fn mermaid_template_er_valid() {
        let template = MermaidTemplates::er_saas_schema();
        assert!(template.contains("erDiagram"));
        assert!(template.contains("TENANT"));
    }

    #[test]
    fn c4_template_context_generates_workspace() {
        let dsl = C4Templates::saas_context("MyApp");
        assert!(dsl.contains("workspace"));
        assert!(dsl.contains("MyApp"));
        assert!(dsl.contains("softwareSystem"));
    }

    #[test]
    fn plantuml_component_template() {
        let puml = PlantUmlTemplates::component_diagram("MySystem", &[("API", "Rust"), ("DB", "PostgreSQL")]);
        assert!(puml.contains("@startuml"));
        assert!(puml.contains("[API]"));
        assert!(puml.contains("Rust"));
    }

    #[test]
    fn make_mermaid_doc_sets_provider() {
        let doc = make_mermaid_doc("Test", DiagramKind::Flowchart, "flowchart TD\n A-->B");
        assert_eq!(doc.provider, ProviderKind::Mermaid);
        assert_eq!(doc.format, DiagramFormat::MermaidMd);
    }

    #[test]
    fn diagram_request_with_format_overrides() {
        let req = DiagramRequest::new("test", DiagramKind::Flowchart)
            .with_format(DiagramFormat::DrawIoXml);
        assert_eq!(req.format, DiagramFormat::DrawIoXml);
    }
}
