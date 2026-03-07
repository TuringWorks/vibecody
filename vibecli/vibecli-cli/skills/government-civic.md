---
triggers: ["government", "civic tech", "GovTech", "e-government", "permitting", "citizen portal", "public records", "FOIA", "government compliance", "municipal", "voting system", "benefits system"]
tools_allowed: ["read_file", "write_file", "bash"]
category: government
---

# Government & Civic Tech Engineering

When working with government and civic technology systems:

1. Design citizen portals with accessibility as a non-negotiable baseline: meet WCAG 2.1 AA at minimum (Section 508 compliance is federal law). Use semantic HTML, provide skip navigation links, ensure all interactive elements are keyboard-operable, maintain a minimum 4.5:1 color contrast ratio, and include ARIA landmarks and live regions. Test with screen readers (NVDA, VoiceOver, JAWS) and automated tools (axe-core) in CI.

2. Build permitting and licensing workflow engines using a configurable state machine: Application Submitted -> UnderReview -> AdditionalInfoRequested -> Approved/Denied -> Issued -> Active -> Expired/Renewed. Define workflow templates per permit type (building, business, event, liquor) with configurable review steps, required documents, fee schedules, and SLA timers. Support parallel review tracks (fire, zoning, health) that must all pass before issuance.

3. Implement public records management with a records retention schedule engine that maps each record type to its legally mandated retention period, disposition method (destroy, transfer to archives, permanent), and triggering event (creation date, case closure, superseded). Automate retention holds for litigation, generate destruction certificates, and maintain a chain-of-custody audit log for every record lifecycle event.

4. Process FOIA (Freedom of Information Act) requests through a dedicated case management system: Request Received -> Acknowledged -> Assigned -> SearchInProgress -> ReviewRedaction -> ResponseDrafted -> Approved -> Released. Track statutory deadlines (20 business days federal, varies by state), manage exemption codes (b1-b9 for federal), support partial releases with redaction tracking, and generate public reading room indexes for proactive disclosure.

5. Design benefits eligibility engines using a rules engine (Drools, OpenFisca, or custom DSL) that encodes eligibility criteria from statute and regulation. Separate the policy rules from the application code so policy analysts can update thresholds (income limits, household size tables, asset caps) without developer intervention. Version rules sets, maintain determination audit trails, and support what-if scenario modeling for policy impact analysis.

6. Build case management systems with a unified case record that links an individual or entity to all related interactions, documents, tasks, and determinations across programs. Support case assignment rules (round-robin, workload-based, geographic), escalation paths, and supervisor review workflows. Implement role-based views so caseworkers see only the data elements their program authorizes.

7. Enable inter-agency data sharing through well-defined APIs with a data governance framework: publish data catalogs with schema definitions, establish data sharing agreements (DSAs) as code-reviewable configuration, implement attribute-based access control (ABAC) that enforces sharing rules at the field level, and log every cross-agency data access for audit. Use standards like NIEM (National Information Exchange Model) for interoperability.

8. Secure election and voting systems with defense-in-depth: air-gap ballot tabulation systems from the internet, implement end-to-end verifiable voting protocols where feasible, maintain immutable audit logs with cryptographic chaining, support risk-limiting audits (RLA) by preserving paper ballot records, and follow EAC (Election Assistance Commission) VVSG guidelines. Conduct independent security assessments and publish results transparently.

9. Build open data portals using DCAT (Data Catalog Vocabulary) and CKAN or Socrata-compatible APIs. Publish datasets in machine-readable formats (CSV, JSON, GeoJSON, Parquet), include comprehensive metadata (update frequency, data dictionary, license, point of contact), automate dataset refresh from source systems, and provide API endpoints with pagination, filtering, and bulk download. Monitor dataset usage analytics to prioritize high-value releases.

10. Implement government payment processing with PCI DSS compliance: use a government-approved payment gateway (Pay.gov for federal, or state equivalents), support ACH, credit/debit cards, and digital wallets. Generate unique transaction reference numbers, provide real-time receipts, reconcile payments against receivables nightly, and handle refunds and chargebacks with full audit trails. Maintain separate trust fund accounting where required.

11. Integrate identity verification using Login.gov patterns for federal systems or state equivalents: support IAL2 (identity assurance level 2) with document verification and selfie matching for high-assurance use cases, and IAL1 with email/phone verification for low-risk services. Implement AAL2 multi-factor authentication (phishing-resistant FIDO2/WebAuthn preferred), support single sign-on across agency applications, and maintain a privacy-preserving architecture that minimizes PII storage.

12. Architect for FedRAMP compliance from the start: deploy to FedRAMP-authorized cloud infrastructure (AWS GovCloud, Azure Government, Google Cloud for Government), implement NIST 800-53 controls mapped to your system's FIPS 199 categorization (Low, Moderate, High), maintain a System Security Plan (SSP), conduct continuous monitoring with automated vulnerability scanning, and prepare for 3PAO assessment. Use OSCAL (Open Security Controls Assessment Language) for machine-readable compliance documentation.

13. Design for multilingual and low-literacy access: implement i18n/l10n from day one with ICU MessageFormat for pluralization and gender-aware translations, provide content in the top languages for the jurisdiction (consult Census LEP data), use plain language (Flesch-Kincaid grade 6-8), and offer alternative formats (audio, large print, TTY). Test with native speakers, not just translation tools.

14. Implement transparent audit logging across all government systems: every data access, modification, and decision point must be logged with who, what, when, where, and why. Store audit logs in an append-only, tamper-evident store. Support legislative and inspector general audit requirements, enable citizen-facing transaction histories, and retain logs per the applicable records retention schedule. Expose audit search interfaces to authorized oversight roles.
