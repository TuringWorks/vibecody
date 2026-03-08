---
triggers: ["X++", "Dynamics 365", "D365 Finance", "D365 Supply Chain", "Dynamics AX", "X++ development", "Finance and Operations"]
tools_allowed: ["read_file", "write_file", "bash"]
category: erp
---

# X++ (Dynamics 365 Finance & Operations)

When developing in X++ for Microsoft Dynamics 365 Finance and Operations:

1. X++ is a managed, object-oriented language that runs on .NET — syntax resembles C#/Java; use Visual Studio with the Dynamics 365 extension for development; deployment uses packages and models.
2. Use the AOT (Application Object Tree) structure: tables, forms, classes, data entities, security roles — extend standard objects with class extensions (`[ExtensionOf(classStr(SalesTable))]`) instead of overlayering to preserve upgradeability.
3. Query data with X++ SQL: `while select salesTable where salesTable.SalesStatus == SalesStatus::Invoiced { info(salesTable.SalesId); }` — X++ SQL is statically typed and checked at compile time; use `QueryBuildDataSource` for dynamic queries.
4. Use data entities for integrations: `public class CustCustomerEntity extends common { ... }` — data entities provide OData endpoints for REST API access; use them for imports, exports, and integrations with Power Platform and Logic Apps.
5. Handle transactions: `ttsbegin; salesTable.SalesStatus = SalesStatus::Confirmed; salesTable.update(); ttscommit;` — `tts` (transaction tracking system) supports nested calls; `ttsabort` rolls back; always pair begin/commit.
6. Use Chain of Command (CoC) for extensions: `[ExtensionOf(classStr(SalesFormLetter))] final class SalesFormLetter_Extension { public void main(Args _args) { next main(_args); // custom logic after } }` — CoC wraps standard methods without modifying them.
7. Create business events for real-time integrations: define event classes that inherit from `BusinessEventsBase`; trigger with `BusinessEventsContract`; subscribe in Power Automate, Logic Apps, or Azure Event Grid.
8. Use SysOperation framework for batch jobs: define `Contract` (parameters), `Service` (logic), `Controller` (orchestration) classes — replaces the legacy RunBase pattern; supports async execution and batch scheduling.
9. Form development: use form patterns (List Page, Details Master, Simple List, Dialog) — each pattern has required controls and layout rules; use `FormDataSource` for data binding; handle events with `[FormEventHandler]` attributes.
10. Security: implement role-based security with duties, privileges, and permissions — `[SysEntryPointAttribute(true)]` marks service operations requiring authorization; use extensible data security policies for row-level filtering.
11. Performance: use `RecordInsertList` for bulk inserts instead of loops; use `set-based operations` (`update_recordset`, `delete_from`, `insert_recordset`) — avoid row-by-row processing; index tables on frequently queried fields.
12. Test with SysTest framework: `[SysTestMethodAttribute] public void testCreateOrder() { ... this.assertEquals(expected, actual); }` — use `SysTestSuite` for grouping; run from Visual Studio Test Explorer.
