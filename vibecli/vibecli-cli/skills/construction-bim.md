---
triggers: ["construction", "BIM", "building information", "project management construction", "cost estimation", "quantity takeoff", "IFC", "scheduling construction", "safety management", "site management"]
tools_allowed: ["read_file", "write_file", "bash"]
category: construction
---

# Construction & BIM Engineering

When working with construction and Building Information Modeling systems:

1. Integrate BIM data by parsing IFC (Industry Foundation Classes) files using libraries like IfcOpenShell or xBIM. Map IFC entities (IfcWall, IfcSlab, IfcDoor) to your domain models, preserving property sets (Pset_*), quantity sets (Qto_*), and spatial hierarchy (IfcProject -> IfcSite -> IfcBuilding -> IfcStorey). Support Revit RVT import via the Forge/APS Model Derivative API for teams that do not export IFC natively.

2. Build project scheduling engines around Critical Path Method (CPM) and optionally PERT for probabilistic durations. Model activities as nodes with dependencies (finish-to-start, start-to-start, finish-to-finish, start-to-finish), compute forward/backward passes for early/late start/finish dates, and identify critical path activities where total float is zero. Expose Gantt chart views and flag schedule slippage against the baseline.

3. Design cost estimation systems with a hierarchical work breakdown structure (WBS) linked to a cost database (RSMeans, proprietary). Each WBS item references unit costs for labor, material, equipment, and subcontractor. Support parametric estimation at early design stages (cost per square foot by building type) and detailed estimation with assembly-level takeoffs as design matures. Version cost estimates against project milestones.

4. Automate quantity takeoff by extracting geometric quantities directly from the BIM model: wall areas, slab volumes, pipe lengths, door counts. Map each BIM element type to a measurement rule (net area, gross area, centerline length). Reconcile model-based quantities against manual takeoffs with a variance report, and re-run automatically when the model updates to keep estimates current.

5. Implement RFI (Request for Information) and submittal workflows as stateful processes: Drafted -> Submitted -> UnderReview -> Responded -> Closed. Track turnaround time against contractual SLAs, link RFIs to specific BIM elements or drawing sheet references, and maintain a full audit trail. Auto-notify responsible parties on status changes and escalate overdue items.

6. Build safety incident tracking with a structured taxonomy: near-miss, first-aid, recordable, lost-time, fatality. Capture incident metadata (location via GPS/BIM zone, time, weather, crew, root cause category). Generate OSHA-reportable logs automatically, compute TRIR (Total Recordable Incident Rate) and DART metrics, and surface leading indicators (inspection deficiency trends) on dashboards.

7. Support progress monitoring with drone/photo AI integration: ingest drone orthomosaics or 360-degree site photos, align them to the BIM model coordinate system, and use computer vision to classify element completion status (not started, in progress, complete). Compare as-built point clouds against the design model to detect deviations, and feed percent-complete data into the schedule and cost modules.

8. Manage equipment fleets with a lifecycle module: track each asset (crane, excavator, generator) by serial number, location (assigned site), maintenance schedule (hours-based or calendar-based), daily usage logs, and cost rates (ownership, operating, idle). Trigger preventive maintenance work orders automatically, and compute equipment utilization rates per project for cost allocation.

9. Handle subcontractor management with a prequalification database (insurance certs, bonding capacity, safety record, past performance ratings), bid management (invitation, submission, leveling, award), and contract administration (pay applications, lien waivers, compliance docs). Automate Subcontractor Default Insurance (SDI) certificate expiration tracking and block payment processing when compliance lapses.

10. Process change orders through a structured workflow: Change Event Identified -> Proposal Requested -> Estimate Submitted -> Negotiated -> Approved/Rejected -> Incorporated into Contract. Link each change order to impacted schedule activities and cost line items, recalculate the contract sum and substantial completion date, and maintain a change order log with cumulative impact summaries.

11. Implement document control with versioning, transmittals, and distribution matrices. Every document (drawing, specification, report, photo) gets a unique identifier, revision history, and approval workflow. Organize by CSI MasterFormat division or Uniclass classification. Support markup and redline workflows with overlay comparison between revisions. Enforce retention policies per project closeout requirements.

12. Enable 4D (schedule-linked) and 5D (cost-linked) BIM visualization by associating each BIM element with its corresponding schedule activity and cost item. Render time-lapse construction sequence animations for 4D, and color-code elements by cost status (under/over budget) for 5D. Use a lightweight web viewer (IFC.js, Autodesk Viewer, Trimble Connect) so stakeholders can explore without desktop BIM software.

13. Design the data architecture around a Common Data Environment (CDE) that serves as the single source of truth. Implement access control by role (owner, architect, GC, subcontractor, inspector) and by project phase. Use webhook-driven integrations to sync data between the CDE and external systems (accounting, ERP, scheduling tools like Primavera P6 or MS Project).

14. Build compliance and inspection modules that map building code requirements (IBC, local amendments) to specific BIM elements and project milestones. Generate inspection checklists automatically from the permit conditions, track inspection results (pass, conditional, fail), link deficiencies to corrective action items, and produce compliance certificates for authority having jurisdiction (AHJ) review.
