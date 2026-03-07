---
triggers: ["legal tech", "contract management", "e-discovery", "legal document", "CLM", "contract lifecycle", "legal AI", "case management", "compliance management", "regulatory tech", "RegTech"]
tools_allowed: ["read_file", "write_file", "bash"]
category: legal
---

# Legal Technology Systems

When working with legal tech, contract management, and compliance software:

1. Design contract lifecycle management (CLM) as a state machine — model contracts through explicit stages (request, draft, negotiate, review, approve, execute, active, amend, renew, expire, terminate) with role-based transition guards, SLA timers on each stage, and a complete audit trail capturing every state change, user action, and timestamp.

2. Build clause extraction using NLP pipelines — implement a multi-stage pipeline: PDF/DOCX parsing to structured text, section segmentation using heading detection and layout analysis, clause classification with fine-tuned transformer models (trained on labeled contract corpora), and entity extraction for key terms (parties, dates, amounts, obligations); store extracted metadata in a structured clause library for reuse and analytics.

3. Design e-discovery pipelines following the EDRM model — implement the Electronic Discovery Reference Model stages: identification (custodian mapping, data source inventory), collection (forensically sound acquisition with hash verification), processing (de-duplication, de-NISTing, text extraction, metadata normalization), review (predictive coding / TAR with active learning), and production (Bates stamping, load file generation in Concordance/Relativity format).

4. Generate legal documents with template engines and merge fields — build a template system supporting conditional sections (if/else on deal terms), repeating blocks (for multiple parties or obligations), computed fields (date calculations, currency conversions), and clause library insertion; enforce version control on templates and maintain a mapping between template versions and generated documents.

5. Implement matter management with financial tracking — model matters (cases, transactions, projects) with hierarchical task structures, link to related contacts/organizations/documents, track time entries and expenses against matter budgets, support matter-level access control (Chinese wall enforcement), and generate matter status reports for client and internal stakeholders.

6. Build compliance rule engines with temporal logic — represent regulatory requirements as versioned rules with effective dates and jurisdictional scope, implement a rule evaluation engine that checks entity states against applicable rules, support rule chaining (one rule triggering evaluation of dependent rules), and generate compliance scorecards with evidence links for audit readiness.

7. Implement regulatory change tracking and impact analysis — ingest regulatory feeds (Federal Register, state legislatures, EU Official Journal), parse change events (new rules, amendments, repeals), map changes to affected internal policies and controls using a regulatory taxonomy, alert compliance officers with impact assessments, and track remediation actions to completion.

8. Support legal billing in LEDES format — generate LEDES 1998B and LEDES XML 2.0 billing files with proper task/activity/expense codes (UTBMS), enforce billing guidelines (block billing detection, rate caps, prohibited charge categories), support e-billing platform submission (Serengeti, Legal Tracker, CounselLink), and implement split billing for multi-matter invoices.

9. Build document comparison and redlining engines — implement character-level and word-level diff algorithms (Myers diff or patience diff) for contract comparison, render changes with tracked-changes markup (insertions, deletions, moves), support three-way comparison (original, counterparty, current), and generate redline summaries highlighting material changes to key commercial terms.

10. Automate privilege review with ML classification — train binary classifiers to identify potentially privileged documents (attorney-client, work product) using features from email metadata (to/from attorneys), document content, and communication patterns; present flagged documents for attorney review with confidence scores; and maintain a privilege log with required fields (date, author, recipient, subject, privilege basis).

11. Integrate court filing systems (e-filing) — implement adapters for electronic court filing systems (Tyler Technologies, File & ServeXpress, PACER), format documents per local court rules (page limits, font requirements, bookmarking), handle filing fees and payment processing, track filing confirmations and deadlines, and support service of process notifications.

12. Design legal hold workflows for litigation readiness — trigger legal holds from matter events or manual initiation, notify custodians with acknowledgment tracking (escalate non-responses), suspend data retention policies for in-scope data sources, track hold scope (custodians, date ranges, data types), support hold release with verification, and generate defensibility reports documenting preservation actions.

13. Implement contract obligation tracking and alerting — extract obligations (payment terms, deliverables, renewal notice periods, reporting requirements) from executed contracts into a structured obligation register, schedule automated reminders at configurable lead times, track fulfillment status, and flag overdue obligations with escalation to responsible parties.

14. Build secure collaboration with ethical wall enforcement — implement information barriers (Chinese walls / ethical walls) that restrict document access, search results, and communication between defined groups (e.g., teams advising opposing parties in M&A), enforce at the data layer (not just UI), audit barrier violations, and support temporary barrier modifications with partner approval.
