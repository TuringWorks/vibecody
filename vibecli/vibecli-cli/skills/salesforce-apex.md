---
triggers: ["Salesforce", "Apex", "salesforce apex", "SOQL", "lightning web component", "LWC", "sfdx", "salesforce trigger", "governor limits", "salesforce flow"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["sf"]
category: salesforce
---

# Salesforce Apex Development

When working with Salesforce Apex development:

1. Bulkify all trigger logic by operating on collections (`Trigger.new`, `Trigger.oldMap`) rather than single records; never use SOQL or DML inside loops to avoid governor limit violations.
2. Use the one-trigger-per-object pattern with a handler class that delegates to domain methods, keeping trigger files minimal and logic testable outside the trigger context.
3. Write SOQL queries with selective filters on indexed fields, use `LIMIT` clauses, and prefer `FOR UPDATE` only when row-locking is required; use SOSL for cross-object text searches instead of multiple SOQL queries with `LIKE`.
4. Implement `Batchable<SObject>` for large data operations, `Queueable` for chained async work with state, and `Schedulable` for cron-driven jobs; always set reasonable `Database.QueryLocator` scopes (200-2000).
5. Build Lightning Web Components with reactive `@api` and `@track` properties, use `wire` adapters for declarative data access, and fall back to imperative `apex` calls only when conditional fetching is needed.
6. Respect governor limits by caching query results in static variables, using `Limits` class methods to monitor consumption at runtime, and leveraging Platform Cache for frequently accessed reference data.
7. Write test classes with `@isTest` that cover bulk scenarios (200+ records), use `Test.startTest()`/`Test.stopTest()` to reset governor limits, and assert specific outcomes rather than just verifying no exceptions were thrown.
8. Store configurable values in Custom Metadata Types (`__mdt`) instead of Custom Settings or hard-coded constants, allowing deployment via packages and change sets without data migration.
9. Publish and subscribe to Platform Events (`__e`) for decoupled integrations; use `EventBus.publish()` with `Database.SaveResult` checks and set replay IDs in subscribers for reliable delivery.
10. Build REST callouts using `HttpRequest`/`HttpResponse` with Named Credentials for auth; implement `HttpCalloutMock` in tests, handle timeouts explicitly, and respect the 120-second callout limit and 100-callout-per-transaction ceiling.
11. Structure SFDX projects with `sfdx-project.json` package directories, use scratch orgs for isolated development, run `sf project deploy start --dry-run` to validate before deploying, and automate CI with `sf project deploy report`.
12. Use `@AuraEnabled(cacheable=true)` for read-only methods consumed by LWC, apply `WITH SECURITY_ENFORCED` or `stripInaccessible()` to enforce FLS, and never expose sensitive fields without explicit permission checks.
