---
triggers: ["manufacturing", "MES", "production planning", "quality management", "SPC", "ERP manufacturing", "bill of materials", "BOM", "work order", "shop floor", "lean manufacturing"]
tools_allowed: ["read_file", "write_file", "bash"]
category: manufacturing
---

# Manufacturing & MES Engineering

When working with manufacturing execution and production systems:

1. Architect MES (Manufacturing Execution System) around the ISA-95 (IEC 62264) functional model: separate Level 4 (ERP/business planning) from Level 3 (manufacturing operations) with well-defined integration interfaces. Implement the core MES functions -- production scheduling, dispatching, execution tracking, data collection, quality management, and performance analysis -- as loosely coupled services that communicate through an event bus or message broker.

2. Model Bills of Materials (BOM) and routings as versioned, effectivity-dated structures. A manufacturing BOM links parent assemblies to child components with quantity-per, unit of measure, and reference designators. Routings define the ordered sequence of operations, each with a work center, setup time, run time per unit, and required tooling. Support engineering change orders (ECOs) that create new BOM/routing revisions with future effectivity dates, preserving history for traceability.

3. Implement production scheduling with MRP (Material Requirements Planning) for material-driven scheduling and APS (Advanced Planning and Scheduling) for constraint-based optimization. MRP explodes demand through the BOM, nets against inventory and open orders, and generates planned purchase and production orders. APS layers on finite capacity constraints (machine hours, labor shifts, tooling availability) to produce feasible schedules with sequence optimization.

4. Design the work order lifecycle as a state machine: Planned -> Released -> Started -> InProgress -> Complete -> Closed. Each work order references a BOM revision and routing revision, carries a target quantity, and tracks actual quantities at each operation (good, scrap, rework). Support work order splitting (for partial completions) and merging (for campaign runs). Record labor, material, and machine time bookings against each operation for cost absorption.

5. Build quality management with Statistical Process Control (SPC) and Statistical Quality Control (SQC): define inspection plans per operation with characteristics (dimensional, visual, functional), sampling rules (AQL-based or 100% inspection), and control chart types (X-bar/R, p-chart, c-chart). Compute Cp/Cpk indices in real time, trigger alerts on out-of-control conditions (Western Electric rules), and auto-generate non-conformance reports when specifications are violated.

6. Collect shop floor data through a multi-source ingestion layer: PLC/SCADA systems via OPC-UA for machine data (cycle counts, temperatures, pressures), operator terminals or tablets for manual entries (start/stop, quality readings, downtime reason codes), barcode/RFID scanners for material tracking, and IoT sensors for environmental monitoring. Normalize all data to a common time-series schema with equipment ID, timestamp, and tag-value pairs.

7. Calculate OEE (Overall Equipment Effectiveness) as Availability x Performance x Quality for each machine, line, and plant. Availability = (Planned Production Time - Downtime) / Planned Production Time. Performance = (Ideal Cycle Time x Total Count) / Run Time. Quality = Good Count / Total Count. Categorize downtime by reason codes (planned maintenance, changeover, breakdown, material shortage) and surface Pareto charts for loss analysis.

8. Implement batch and lot traceability with forward and backward trace capability. Assign unique lot numbers at material receipt and at each production step. Record lot-to-lot genealogy: which input lots were consumed to produce which output lots, at which operation, by which operator, on which equipment, at what time. Support single-click trace queries that answer "where did this lot go?" (forward) and "what went into this lot?" (backward) for recall scenarios.

9. Manage non-conformance with a structured CAPA (Corrective and Preventive Action) workflow: Non-Conformance Detected -> Containment -> Root Cause Analysis (5-Why, Ishikawa, 8D) -> Corrective Action Defined -> Implemented -> Effectiveness Verified -> Closed. Link non-conformances to specific lots, operations, and equipment. Track cost of quality (internal failure, external failure, appraisal, prevention) as first-class metrics.

10. Integrate with ERP systems (SAP, Oracle, Microsoft Dynamics) through a middleware layer that handles bidirectional data flow: ERP sends planned orders, BOM/routing masters, and material availability; MES sends back production confirmations, material consumption postings, quality results, and labor/machine time bookings. Use idempotent message processing and store-and-forward queuing to handle ERP downtime without losing shop floor transactions.

11. Track lean manufacturing metrics and support continuous improvement: monitor takt time (available production time / customer demand rate), cycle time per operation, lead time (order to delivery), WIP (work in progress) levels, and first-pass yield. Implement digital Kanban boards for pull-based production control, support value stream mapping data collection, and provide Kaizen event tracking with before/after metric comparison dashboards.

12. Manage tooling and dies with a lifecycle module: register each tool with specifications (material, tolerances, expected life in cycles or hours), track usage counts per production run, schedule preventive maintenance or replacement at defined thresholds, record sharpening/refurbishment history, and allocate tool costs to production orders. Flag when a tool approaches end-of-life and auto-generate procurement or refurbishment requests.

13. Design for regulatory compliance in regulated industries (FDA 21 CFR Part 11, EU GMP Annex 11): implement electronic signatures with meaning (authored, reviewed, approved), maintain complete audit trails that are tamper-evident and cannot be disabled, enforce access controls with user authentication, and validate the system per GAMP 5 guidelines. Ensure all quality-critical records are retrievable for the mandated retention period.

14. Implement production dashboards and Andon systems for real-time shop floor visibility: display current order status, machine states (running, idle, down, changeover), OEE gauges, quality alerts, and schedule adherence on large-format screens at the production line. Support escalation workflows where an Andon alert triggers team leader notification, and if unresolved within a configurable window, escalates to the shift supervisor and then plant manager.
