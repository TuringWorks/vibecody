---
triggers: ["Salesforce admin", "salesforce object", "salesforce flow builder", "salesforce permission", "salesforce report", "salesforce sandbox", "salesforce change set"]
tools_allowed: ["read_file", "write_file", "bash"]
category: salesforce
---

# Salesforce Administration & Configuration

When working with Salesforce admin and configuration tasks:

1. Design custom objects with clear naming conventions (`Project__c`), always add a description, and set the appropriate sharing model (Private, Public Read Only, or Public Read/Write) based on data sensitivity before creating fields.
2. Create fields with the most restrictive type that fits the data (picklist over text, number over text for numeric values), set field-level help text, and mark fields required at the page layout level rather than universally when only certain profiles need enforcement.
3. Build page layouts with logical sections, place critical fields above the fold, and use Dynamic Forms on Lightning Record Pages to conditionally show field sections based on record type or field values instead of maintaining multiple page layouts.
4. Write validation rules with `ISBLANK()`, `REGEX()`, and cross-object formulas to enforce data quality; combine conditions with `AND()`/`OR()` and always include a clear `Error Message` with the field location so users know exactly what to fix.
5. Build Flows instead of Process Builder or Workflow Rules for all new automation; use Record-Triggered Flows with entry conditions, Before-Save for field updates (no DML cost), and After-Save only when child records or external calls are needed.
6. Assign permissions through Permission Sets and Permission Set Groups rather than Profiles; create task-based permission sets (e.g., "Invoice Manager") and combine them into groups to simplify assignment and auditing.
7. Configure Profiles only for login hours/IP ranges, default record types, and page layout assignments; move all object and field permissions into Permission Sets to enable flexible, composable access control.
8. Build reports using the appropriate type (Tabular for lists, Summary for grouping, Matrix for cross-tab, Joined for multi-block); add report filters at the report level rather than dashboard level for consistent results, and schedule reports for stakeholder delivery.
9. Design dashboards with a mix of chart types (donut for composition, bar for comparison, gauge for KPIs), keep them under 20 components for performance, and set a running user with appropriate visibility or use "Run as logged-in user" for row-level security.
10. Use Data Loader for bulk operations over 50K records (insert, update, upsert, delete, export), always perform a backup export before mass updates, and use external ID fields for upsert operations to prevent duplicates.
11. Manage sandboxes by using Developer sandboxes for feature work, Partial Copy for QA with representative data, and Full Copy only for UAT/staging; refresh on a schedule and use sandbox-specific email settings to prevent accidental emails to production contacts.
12. Deploy with Change Sets for simple org-to-org moves by adding all dependencies (objects, fields, layouts, flows, classes) in the correct order; for repeatable deployments, migrate to SFDX source format and use `sf project deploy start` with a manifest (`package.xml`).
