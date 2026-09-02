//! Draw.io starter diagrams.
//!
//! # Why this file exists
//!
//! The Templates tab advertised eight diagrams — "Microservices Architecture",
//! "C4 Container", "REST API Sequence" — and `get_drawio_template` returned the
//! same thing for every one of them: a single rounded rectangle containing the
//! template's own name. Picking "Microservices Architecture" gave you one box
//! that said *Microservices Architecture*. The tab was a menu of promises with
//! nothing behind it, which is worse than not offering templates: a user who
//! clicks one concludes the editor is broken.
//!
//! Each template below is a real diagram — laid out, connected, and openable —
//! that a person can rename and extend. They are deliberately small (8–14
//! shapes): a starter someone edits beats a finished picture they delete.
//!
//! # Coordinates
//!
//! draw.io's y-axis grows downward from the top-left. Everything here is laid
//! out on a 40 px grid starting at (40, 40) so the diagram opens near the origin
//! rather than somewhere the user has to scroll to find.

/// A template's stable identifier and the shape of diagram it produces.
pub struct Template {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    /// One line saying what the diagram shows, for the card in the panel.
    pub summary: &'static str,
}

/// Every template the panel may offer.
///
/// The panel reads this list rather than keeping its own copy — a hard-coded
/// TypeScript array is how the tab came to advertise eight templates that the
/// backend did not have.
pub const TEMPLATES: &[Template] = &[
    Template {
        id: "microservices",
        label: "Microservices Architecture",
        kind: "architecture",
        summary: "Gateway, three services, their datastores and a message bus",
    },
    Template {
        id: "ci_cd",
        label: "CI/CD Pipeline",
        kind: "flowchart",
        summary: "Commit through build, test, and gated deploy to production",
    },
    Template {
        id: "er_saas",
        label: "SaaS ERD",
        kind: "entity_relationship",
        summary: "Accounts, users, subscriptions, plans and invoices",
    },
    Template {
        id: "c4_context",
        label: "C4 — System Context",
        kind: "c4_context",
        summary: "One system, its people, and the systems it talks to",
    },
    Template {
        id: "c4_container",
        label: "C4 — Container",
        kind: "c4_container",
        summary: "Web app, API, worker and database inside one boundary",
    },
    Template {
        id: "api_sequence",
        label: "REST API Sequence",
        kind: "sequence",
        summary: "Client → gateway → service → database, with the return path",
    },
    Template {
        id: "state_order",
        label: "Order State Machine",
        kind: "state_machine",
        summary: "Placed through delivered, with the cancel and refund branches",
    },
    Template {
        id: "domain_model",
        label: "Domain Class Diagram",
        kind: "class_diagram",
        summary: "Customer, Order, OrderLine and Product with multiplicities",
    },
];

/// Styles, named once so the eight diagrams look like one family rather than
/// eight people's defaults.
mod style {
    pub const BOX: &str = "rounded=1;whiteSpace=wrap;html=1;arcSize=8;fillColor=#1f2937;strokeColor=#4b5563;fontColor=#e5e7eb;fontSize=12;";
    pub const ACCENT: &str = "rounded=1;whiteSpace=wrap;html=1;arcSize=8;fillColor=#1e3a5f;strokeColor=#3b82f6;fontColor=#dbeafe;fontSize=12;";
    pub const STORE: &str = "shape=cylinder3;boundedLbl=1;backgroundOutline=1;size=8;whiteSpace=wrap;html=1;fillColor=#1f2937;strokeColor=#4b5563;fontColor=#e5e7eb;fontSize=11;";
    pub const ACTOR: &str = "shape=umlActor;verticalLabelPosition=bottom;verticalAlign=top;html=1;strokeColor=#9ca3af;fontColor=#e5e7eb;fontSize=11;";
    pub const NOTE: &str = "shape=note;whiteSpace=wrap;html=1;size=14;fillColor=#292524;strokeColor=#57534e;fontColor=#d6d3d1;fontSize=11;align=left;verticalAlign=top;";
    pub const BOUNDARY: &str = "rounded=1;whiteSpace=wrap;html=1;dashed=1;fillColor=none;strokeColor=#6b7280;fontColor=#9ca3af;verticalAlign=top;fontSize=11;";
    pub const EDGE: &str = "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;strokeColor=#6b7280;fontColor=#d1d5db;fontSize=10;";
    pub const EDGE_DASHED: &str = "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;dashed=1;strokeColor=#6b7280;fontColor=#d1d5db;fontSize=10;";
    pub const LIFELINE: &str = "shape=umlLifeline;perimeter=lifelinePerimeter;whiteSpace=wrap;html=1;container=0;dropTarget=0;collapsible=0;recursiveResize=0;outlineConnect=0;fillColor=#1f2937;strokeColor=#4b5563;fontColor=#e5e7eb;fontSize=11;";
    pub const MSG: &str = "html=1;verticalAlign=bottom;endArrow=block;strokeColor=#9ca3af;fontColor=#e5e7eb;fontSize=10;";
    pub const MSG_RETURN: &str = "html=1;verticalAlign=bottom;endArrow=open;dashed=1;strokeColor=#9ca3af;fontColor=#e5e7eb;fontSize=10;";
    pub const STATE: &str = "rounded=1;whiteSpace=wrap;html=1;arcSize=40;fillColor=#1f2937;strokeColor=#4b5563;fontColor=#e5e7eb;fontSize=12;";
    pub const START: &str = "ellipse;html=1;fillColor=#e5e7eb;strokeColor=#e5e7eb;";
    pub const END: &str = "ellipse;shape=endState;html=1;fillColor=#e5e7eb;strokeColor=#e5e7eb;";
    pub const CLASS: &str = "swimlane;fontStyle=1;childLayout=stackLayout;horizontal=1;startSize=26;horizontalStack=0;resizeParent=1;resizeParentMax=0;html=1;whiteSpace=wrap;fillColor=#1f2937;strokeColor=#4b5563;fontColor=#e5e7eb;fontSize=12;";
    pub const FIELD: &str = "text;strokeColor=none;fillColor=none;align=left;verticalAlign=middle;spacingLeft=6;html=1;fontColor=#d1d5db;fontSize=11;";
    pub const ENTITY: &str = "swimlane;fontStyle=1;childLayout=stackLayout;horizontal=1;startSize=26;horizontalStack=0;resizeParent=1;resizeParentMax=0;html=1;whiteSpace=wrap;fillColor=#1e3a5f;strokeColor=#3b82f6;fontColor=#dbeafe;fontSize=12;";
    pub const DECISION: &str = "rhombus;whiteSpace=wrap;html=1;fillColor=#1f2937;strokeColor=#4b5563;fontColor=#e5e7eb;fontSize=11;";
}

/// XML-escape a label.
///
/// draw.io labels may contain HTML — `<b>`, `<br/>`, `<font>` — but the label
/// lives in an XML *attribute*, so the markup has to be escaped there and is
/// unescaped by the parser before draw.io renders it. Writing it raw produces a
/// file that is not well-formed XML at all: draw.io shows an empty canvas and
/// says nothing, and the string-counting tests in this module happily passed
/// two such templates until a real XML parser was pointed at them.
fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A vertex.
fn node(id: &str, value: &str, style: &str, x: i32, y: i32, w: i32, h: i32) -> String {
    let value = esc(value);
    format!(
        "<mxCell id=\"{id}\" value=\"{value}\" style=\"{style}\" vertex=\"1\" parent=\"1\">\
         <mxGeometry x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" as=\"geometry\"/></mxCell>"
    )
}

/// A vertex inside another vertex — a class field, an entity column.
fn child(id: &str, value: &str, style: &str, parent: &str, y: i32, w: i32, h: i32) -> String {
    let value = esc(value);
    format!(
        "<mxCell id=\"{id}\" value=\"{value}\" style=\"{style}\" vertex=\"1\" parent=\"{parent}\">\
         <mxGeometry y=\"{y}\" width=\"{w}\" height=\"{h}\" as=\"geometry\"/></mxCell>"
    )
}

/// An edge.
fn edge(id: &str, value: &str, style: &str, source: &str, target: &str) -> String {
    let value = esc(value);
    format!(
        "<mxCell id=\"{id}\" value=\"{value}\" style=\"{style}\" edge=\"1\" parent=\"1\" \
         source=\"{source}\" target=\"{target}\"><mxGeometry relative=\"1\" as=\"geometry\"/></mxCell>"
    )
}

/// A sequence-diagram message: an edge pinned to two absolute points, because
/// lifeline messages are positioned by y rather than by which cell they touch.
fn message(id: &str, value: &str, style: &str, x1: i32, x2: i32, y: i32) -> String {
    let value = esc(value);
    format!(
        "<mxCell id=\"{id}\" value=\"{value}\" style=\"{style}\" edge=\"1\" parent=\"1\">\
         <mxGeometry relative=\"1\" as=\"geometry\">\
         <mxPoint x=\"{x1}\" y=\"{y}\" as=\"sourcePoint\"/>\
         <mxPoint x=\"{x2}\" y=\"{y}\" as=\"targetPoint\"/></mxGeometry></mxCell>"
    )
}

/// Wrap cells in the `mxfile` envelope draw.io writes itself.
///
/// The envelope (not a bare `mxGraphModel`) is what a `.drawio` file on disk
/// contains, so a template saved straight from the panel is byte-shaped like
/// one draw.io produced — and opens in the desktop app without conversion.
fn document(page_name: &str, cells: &str) -> String {
    format!(
        "<mxfile host=\"VibeCody\" type=\"device\">\
         <diagram name=\"{page_name}\" id=\"page-1\">\
         <mxGraphModel dx=\"1100\" dy=\"800\" grid=\"1\" gridSize=\"10\" guides=\"1\" \
         tooltips=\"1\" connect=\"1\" arrows=\"1\" fold=\"1\" page=\"1\" pageScale=\"1\" \
         pageWidth=\"1169\" pageHeight=\"826\" math=\"0\" shadow=\"0\">\
         <root><mxCell id=\"0\"/><mxCell id=\"1\" parent=\"0\"/>{cells}</root>\
         </mxGraphModel></diagram></mxfile>"
    )
}

fn microservices() -> String {
    let cells = [
        node("gw", "API Gateway", style::ACCENT, 440, 40, 200, 50),
        node("orders", "Orders Service", style::BOX, 200, 200, 180, 50),
        node("billing", "Billing Service", style::BOX, 440, 200, 180, 50),
        node("catalog", "Catalog Service", style::BOX, 680, 200, 180, 50),
        node("odb", "orders_db", style::STORE, 220, 340, 140, 70),
        node("bdb", "billing_db", style::STORE, 460, 340, 140, 70),
        node("cdb", "catalog_db", style::STORE, 700, 340, 140, 70),
        node("bus", "Message Bus\n(order.placed, payment.settled)", style::ACCENT, 340, 480, 380, 60),
        node(
            "note",
            "Replace the names with your own services.\nDashed edges are asynchronous events.",
            style::NOTE,
            900,
            40,
            230,
            80,
        ),
        edge("e1", "REST", style::EDGE, "gw", "orders"),
        edge("e2", "REST", style::EDGE, "gw", "billing"),
        edge("e3", "REST", style::EDGE, "gw", "catalog"),
        edge("e4", "", style::EDGE, "orders", "odb"),
        edge("e5", "", style::EDGE, "billing", "bdb"),
        edge("e6", "", style::EDGE, "catalog", "cdb"),
        edge("e7", "publishes", style::EDGE_DASHED, "orders", "bus"),
        edge("e8", "subscribes", style::EDGE_DASHED, "bus", "billing"),
    ]
    .concat();
    document("Microservices", &cells)
}

fn ci_cd() -> String {
    let cells = [
        node("start", "", style::START, 60, 210, 30, 30),
        node("commit", "Commit\npushed", style::BOX, 130, 195, 130, 60),
        node("build", "Build", style::BOX, 300, 195, 130, 60),
        node("test", "Unit +\nintegration tests", style::BOX, 470, 195, 150, 60),
        node("gate", "Tests\npass?", style::DECISION, 670, 185, 120, 80),
        node("stage", "Deploy to\nstaging", style::ACCENT, 840, 195, 140, 60),
        node("approve", "Manual\napproval", style::DECISION, 840, 330, 140, 80),
        node("prod", "Deploy to\nproduction", style::ACCENT, 840, 460, 140, 60),
        node("fail", "Notify author\n· block merge", style::BOX, 670, 330, 130, 60),
        node("done", "", style::END, 895, 570, 30, 30),
        edge("c1", "", style::EDGE, "start", "commit"),
        edge("c2", "", style::EDGE, "commit", "build"),
        edge("c3", "", style::EDGE, "build", "test"),
        edge("c4", "", style::EDGE, "test", "gate"),
        edge("c5", "yes", style::EDGE, "gate", "stage"),
        edge("c6", "no", style::EDGE, "gate", "fail"),
        edge("c7", "", style::EDGE, "stage", "approve"),
        edge("c8", "approved", style::EDGE, "approve", "prod"),
        edge("c9", "", style::EDGE, "prod", "done"),
    ]
    .concat();
    document("CI/CD Pipeline", &cells)
}

fn er_saas() -> String {
    // Entities are swimlanes; each column is a stacked child, so draw.io's
    // stack layout keeps them aligned when a row is added or removed.
    let account = [
        node("acct", "account", style::ENTITY, 60, 60, 200, 138),
        child("acct_id", "id  PK", style::FIELD, "acct", 26, 200, 26),
        child("acct_name", "name", style::FIELD, "acct", 52, 200, 26),
        child("acct_created", "created_at", style::FIELD, "acct", 78, 200, 26),
        child("acct_plan", "plan_id  FK", style::FIELD, "acct", 104, 200, 26),
    ]
    .concat();
    let user = [
        node("usr", "user", style::ENTITY, 340, 60, 200, 138),
        child("usr_id", "id  PK", style::FIELD, "usr", 26, 200, 26),
        child("usr_acct", "account_id  FK", style::FIELD, "usr", 52, 200, 26),
        child("usr_email", "email  UNIQUE", style::FIELD, "usr", 78, 200, 26),
        child("usr_role", "role", style::FIELD, "usr", 104, 200, 26),
    ]
    .concat();
    let plan = [
        node("plan", "plan", style::ENTITY, 60, 300, 200, 112),
        child("plan_id", "id  PK", style::FIELD, "plan", 26, 200, 26),
        child("plan_name", "name", style::FIELD, "plan", 52, 200, 26),
        child("plan_price", "price_cents", style::FIELD, "plan", 78, 200, 26),
    ]
    .concat();
    let sub = [
        node("sub", "subscription", style::ENTITY, 340, 300, 200, 138),
        child("sub_id", "id  PK", style::FIELD, "sub", 26, 200, 26),
        child("sub_acct", "account_id  FK", style::FIELD, "sub", 52, 200, 26),
        child("sub_status", "status", style::FIELD, "sub", 78, 200, 26),
        child("sub_period", "period_end", style::FIELD, "sub", 104, 200, 26),
    ]
    .concat();
    let invoice = [
        node("inv", "invoice", style::ENTITY, 620, 300, 200, 138),
        child("inv_id", "id  PK", style::FIELD, "inv", 26, 200, 26),
        child("inv_sub", "subscription_id  FK", style::FIELD, "inv", 52, 200, 26),
        child("inv_total", "total_cents", style::FIELD, "inv", 78, 200, 26),
        child("inv_paid", "paid_at", style::FIELD, "inv", 104, 200, 26),
    ]
    .concat();
    let edges = [
        edge("r1", "1 : N", style::EDGE, "acct", "usr"),
        edge("r2", "1 : N", style::EDGE, "plan", "acct"),
        edge("r3", "1 : N", style::EDGE, "acct", "sub"),
        edge("r4", "1 : N", style::EDGE, "sub", "inv"),
    ]
    .concat();
    document(
        "SaaS ERD",
        &[account, user, plan, sub, invoice, edges].concat(),
    )
}

fn c4_context() -> String {
    let cells = [
        node("cust", "Customer", style::ACTOR, 120, 60, 30, 60),
        node("admin", "Support Agent", style::ACTOR, 120, 260, 30, 60),
        node(
            "sys",
            "<b>Your System</b><br/><font style=\"font-size:10px\">[Software System]</font>",
            style::ACCENT,
            360,
            140,
            220,
            90,
        ),
        node(
            "pay",
            "<b>Payment Provider</b><br/><font style=\"font-size:10px\">[External]</font>",
            style::BOX,
            700,
            60,
            200,
            80,
        ),
        node(
            "mail",
            "<b>Email Service</b><br/><font style=\"font-size:10px\">[External]</font>",
            style::BOX,
            700,
            240,
            200,
            80,
        ),
        node(
            "note",
            "C4 level 1: people and systems only.\nNo containers, no technology choices —\nthose belong on the Container diagram.",
            style::NOTE,
            360,
            320,
            280,
            90,
        ),
        edge("x1", "places orders", style::EDGE, "cust", "sys"),
        edge("x2", "resolves tickets", style::EDGE, "admin", "sys"),
        edge("x3", "takes payment via", style::EDGE, "sys", "pay"),
        edge("x4", "sends receipts via", style::EDGE, "sys", "mail"),
    ]
    .concat();
    document("C4 — System Context", &cells)
}

fn c4_container() -> String {
    let cells = [
        node("cust", "Customer", style::ACTOR, 90, 180, 30, 60),
        node("bound", "Your System", style::BOUNDARY, 240, 60, 620, 400),
        node(
            "spa",
            "<b>Web App</b><br/><font style=\"font-size:10px\">[React]</font>",
            style::ACCENT,
            280,
            120,
            180,
            70,
        ),
        node(
            "api",
            "<b>API</b><br/><font style=\"font-size:10px\">[Rust · axum]</font>",
            style::ACCENT,
            540,
            120,
            180,
            70,
        ),
        node(
            "worker",
            "<b>Worker</b><br/><font style=\"font-size:10px\">[background jobs]</font>",
            style::ACCENT,
            540,
            260,
            180,
            70,
        ),
        node(
            "db",
            "Database\n[PostgreSQL]",
            style::STORE,
            290,
            270,
            160,
            80,
        ),
        node(
            "note",
            "C4 level 2: one box per separately\ndeployable/runnable thing, each\nlabelled with its technology.",
            style::NOTE,
            900,
            60,
            240,
            90,
        ),
        edge("y1", "HTTPS", style::EDGE, "cust", "spa"),
        edge("y2", "JSON/HTTPS", style::EDGE, "spa", "api"),
        edge("y3", "reads/writes", style::EDGE, "api", "db"),
        edge("y4", "enqueues", style::EDGE_DASHED, "api", "worker"),
        edge("y5", "reads/writes", style::EDGE, "worker", "db"),
    ]
    .concat();
    document("C4 — Container", &cells)
}

fn api_sequence() -> String {
    // Lifelines are tall boxes; messages are edges pinned to y positions
    // between the lifeline x-centres.
    let lifelines = [
        node("l_client", "Client", style::LIFELINE, 80, 40, 120, 460),
        node("l_gw", "API Gateway", style::LIFELINE, 300, 40, 120, 460),
        node("l_svc", "Order Service", style::LIFELINE, 520, 40, 120, 460),
        node("l_db", "Database", style::LIFELINE, 740, 40, 120, 460),
    ]
    .concat();
    // Lifeline x-centres: 140, 360, 580, 800.
    let msgs = [
        message("m1", "POST /orders", style::MSG, 140, 360, 150),
        message("m2", "validate token", style::MSG, 360, 360, 190),
        message("m3", "createOrder(cmd)", style::MSG, 360, 580, 230),
        message("m4", "INSERT order", style::MSG, 580, 800, 270),
        message("m5", "order id", style::MSG_RETURN, 800, 580, 310),
        message("m6", "201 Created", style::MSG_RETURN, 580, 360, 350),
        message("m7", "201 + Location", style::MSG_RETURN, 360, 140, 390),
        node(
            "note",
            "Dashed arrows are returns.\nDrag a message up or down to reorder it.",
            style::NOTE,
            900,
            140,
            230,
            70,
        ),
    ]
    .concat();
    document("REST API Sequence", &[lifelines, msgs].concat())
}

fn state_order() -> String {
    let cells = [
        node("s0", "", style::START, 60, 215, 30, 30),
        node("placed", "Placed", style::STATE, 130, 200, 130, 60),
        node("paid", "Paid", style::STATE, 310, 200, 130, 60),
        node("shipped", "Shipped", style::STATE, 490, 200, 130, 60),
        node("delivered", "Delivered", style::STATE, 670, 200, 130, 60),
        node("cancelled", "Cancelled", style::STATE, 310, 360, 130, 60),
        node("refunded", "Refunded", style::STATE, 490, 360, 130, 60),
        node("end", "", style::END, 855, 215, 30, 30),
        node("end2", "", style::END, 855, 375, 30, 30),
        edge("t0", "", style::EDGE, "s0", "placed"),
        edge("t1", "payment settled", style::EDGE, "placed", "paid"),
        edge("t2", "dispatched", style::EDGE, "paid", "shipped"),
        edge("t3", "signed for", style::EDGE, "shipped", "delivered"),
        edge("t4", "", style::EDGE, "delivered", "end"),
        edge("t5", "cancelled before dispatch", style::EDGE, "placed", "cancelled"),
        edge("t6", "refund issued", style::EDGE, "cancelled", "refunded"),
        edge("t7", "returned", style::EDGE, "delivered", "refunded"),
        edge("t8", "", style::EDGE, "refunded", "end2"),
    ]
    .concat();
    document("Order State Machine", &cells)
}

fn domain_model() -> String {
    let customer = [
        node("cCust", "Customer", style::CLASS, 60, 60, 200, 112),
        child("cc1", "+ id: Uuid", style::FIELD, "cCust", 26, 200, 26),
        child("cc2", "+ email: String", style::FIELD, "cCust", 52, 200, 26),
        child("cc3", "+ placeOrder(): Order", style::FIELD, "cCust", 78, 200, 26),
    ]
    .concat();
    let order = [
        node("cOrd", "Order", style::CLASS, 380, 60, 200, 138),
        child("co1", "+ id: Uuid", style::FIELD, "cOrd", 26, 200, 26),
        child("co2", "+ placedAt: DateTime", style::FIELD, "cOrd", 52, 200, 26),
        child("co3", "+ status: OrderStatus", style::FIELD, "cOrd", 78, 200, 26),
        child("co4", "+ total(): Money", style::FIELD, "cOrd", 104, 200, 26),
    ]
    .concat();
    let line = [
        node("cLine", "OrderLine", style::CLASS, 380, 300, 200, 112),
        child("cl1", "+ quantity: u32", style::FIELD, "cLine", 26, 200, 26),
        child("cl2", "+ unitPrice: Money", style::FIELD, "cLine", 52, 200, 26),
        child("cl3", "+ subtotal(): Money", style::FIELD, "cLine", 78, 200, 26),
    ]
    .concat();
    let product = [
        node("cProd", "Product", style::CLASS, 700, 300, 200, 112),
        child("cp1", "+ sku: String", style::FIELD, "cProd", 26, 200, 26),
        child("cp2", "+ name: String", style::FIELD, "cProd", 52, 200, 26),
        child("cp3", "+ price: Money", style::FIELD, "cProd", 78, 200, 26),
    ]
    .concat();
    let edges = [
        edge("d1", "1        0..*", style::EDGE, "cCust", "cOrd"),
        edge("d2", "1        1..*", style::EDGE, "cOrd", "cLine"),
        edge("d3", "0..*        1", style::EDGE, "cLine", "cProd"),
    ]
    .concat();
    document(
        "Domain Model",
        &[customer, order, line, product, edges].concat(),
    )
}

/// The diagram for `id`, or `None` when no template has that id.
///
/// `None` rather than a placeholder: a caller asking for a template that does
/// not exist should be told so, not handed a box with the id written in it and
/// left to conclude the editor is broken.
pub fn template_xml(id: &str) -> Option<String> {
    Some(match id {
        "microservices" => microservices(),
        "ci_cd" => ci_cd(),
        "er_saas" => er_saas(),
        "c4_context" => c4_context(),
        "c4_container" => c4_container(),
        "api_sequence" => api_sequence(),
        "state_order" => state_order(),
        "domain_model" => domain_model(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_advertised_template_has_a_diagram() {
        // The tab offered eight templates and the backend had none of them.
        // This is the assertion that would have caught it.
        for t in TEMPLATES {
            assert!(
                template_xml(t.id).is_some(),
                "template `{}` is offered in the panel but has no diagram",
                t.id
            );
        }
    }

    #[test]
    fn an_unknown_id_is_none_not_a_placeholder() {
        assert!(template_xml("no-such-template").is_none());
    }

    #[test]
    fn every_template_is_a_real_diagram_not_a_labelled_box() {
        // The defect being pinned: each template used to be one rounded
        // rectangle containing its own name. A diagram worth offering has
        // several shapes and at least one connection between them.
        for t in TEMPLATES {
            let xml = template_xml(t.id).expect("checked above");
            let vertices = xml.matches("vertex=\"1\"").count();
            let edges = xml.matches("edge=\"1\"").count();
            assert!(
                vertices >= 4,
                "template `{}` has {vertices} shapes — that is a placeholder, not a diagram",
                t.id
            );
            assert!(
                edges >= 3,
                "template `{}` has {edges} connections — shapes with nothing joining them \
                 are not a diagram",
                t.id
            );
        }
    }

    /// Parse each template with a real XML parser.
    ///
    /// This is the test that matters. The structural checks below count strings,
    /// and string counting passed `c4_context` and `c4_container` while both
    /// were **invalid XML** — their labels carried raw `<b>` and
    /// `<font style="…">` inside an XML attribute. draw.io renders a malformed
    /// file as an empty canvas and reports nothing, so the only symptom would
    /// have been a user saying the template does not work.
    #[test]
    fn every_template_parses_as_xml() {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        for t in TEMPLATES {
            let xml = template_xml(t.id).expect("every template has a diagram");
            let mut reader = Reader::from_str(&xml);
            reader.config_mut().check_end_names = true;
            let mut depth = 0i32;
            let mut buf = Vec::new();
            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Start(_)) => depth += 1,
                    Ok(Event::End(_)) => depth -= 1,
                    Ok(Event::Eof) => break,
                    Ok(_) => {}
                    Err(e) => panic!(
                        "template `{}` is not well-formed XML at byte {}: {e}",
                        t.id,
                        reader.buffer_position()
                    ),
                }
                buf.clear();
            }
            assert_eq!(depth, 0, "template `{}` has unbalanced elements", t.id);
        }
    }

    #[test]
    fn a_label_containing_markup_is_escaped() {
        // draw.io labels may be HTML, but the label lives in an XML attribute,
        // so the markup must be escaped there.
        let cell = node("x", "<b>Bold</b>", "style", 0, 0, 10, 10);
        assert!(cell.contains("&lt;b&gt;Bold&lt;/b&gt;"), "{cell}");
        assert!(!cell.contains("value=\"<b>"), "raw markup in an attribute: {cell}");
    }

    #[test]
    fn every_template_is_well_formed_enough_to_open() {
        // draw.io renders nothing at all for malformed XML, and the failure is
        // silent — a blank canvas with no error. Cheap structural checks catch
        // the mistakes that actually happen when hand-writing these.
        for t in TEMPLATES {
            let xml = template_xml(t.id).expect("checked above");
            assert!(xml.starts_with("<mxfile"), "{}: no mxfile envelope", t.id);
            assert!(xml.ends_with("</mxfile>"), "{}: envelope not closed", t.id);
            // Every cell this module emits carries a child `<mxGeometry>`, so
            // it has a real closing tag — except the two self-closing root
            // cells. Anything else means a tag was dropped while editing.
            let opens = xml.matches("<mxCell").count();
            let closes = xml.matches("</mxCell>").count();
            let self_closing = xml.matches("<mxCell id=\"0\"/>").count()
                + xml.matches("<mxCell id=\"1\" parent=\"0\"/>").count();
            assert_eq!(
                opens,
                closes + self_closing,
                "{}: {opens} <mxCell> against {closes} closes and {self_closing} self-closing",
                t.id
            );
            assert!(
                xml.contains("<mxCell id=\"0\"/>") && xml.contains("<mxCell id=\"1\" parent=\"0\"/>"),
                "{}: missing the root cells every draw.io model needs",
                t.id
            );
        }
    }

    #[test]
    fn no_edge_points_at_a_cell_that_does_not_exist() {
        // A dangling source/target is the one hand-authoring mistake draw.io
        // does not report: the edge simply vanishes, and the diagram looks
        // subtly wrong rather than broken.
        for t in TEMPLATES {
            let xml = template_xml(t.id).expect("checked above");
            let ids: Vec<&str> = xml
                .match_indices("<mxCell id=\"")
                .map(|(i, m)| {
                    let rest = &xml[i + m.len()..];
                    &rest[..rest.find('"').unwrap_or(0)]
                })
                .collect();
            for attr in ["source=\"", "target=\""] {
                for (i, m) in xml.match_indices(attr) {
                    let rest = &xml[i + m.len()..];
                    let referenced = &rest[..rest.find('"').unwrap_or(0)];
                    assert!(
                        ids.contains(&referenced),
                        "template `{}` has an edge pointing at `{referenced}`, which is not a cell",
                        t.id
                    );
                }
            }
        }
    }

    /// Not an assertion — writes each template out so an independent XML
    /// parser can check them. Ignored by default; run it with
    /// `--ignored` and `DRAWIO_DUMP_DIR` set to a directory.
    #[test]
    #[ignore]
    fn dump_templates_for_external_validation() {
        let dir = std::env::var("DRAWIO_DUMP_DIR").expect("set DRAWIO_DUMP_DIR");
        for t in TEMPLATES {
            let xml = template_xml(t.id).expect("every template has a diagram");
            std::fs::write(format!("{dir}/{}.drawio", t.id), xml).unwrap();
        }
        println!("wrote {} templates to {dir}", TEMPLATES.len());
    }

    #[test]
    fn template_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for t in TEMPLATES {
            assert!(seen.insert(t.id), "duplicate template id `{}`", t.id);
        }
    }
}
