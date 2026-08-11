---
name: "Customs Brokerage and Clearance"
description: "Customs Brokerage and Clearance: Compose this pack with the international-trade overlay and jurisdiction-specific customs law. Use when the task involves customs brokerage and clearance, customs brokerage, clearance."
category: industry
triggers: ["customs brokerage and clearance", "customs brokerage", "clearance"]
tools_allowed: ["read_file", "write_file"]
---

# Customs Brokerage and Clearance

Compose this pack with the international-trade overlay and jurisdiction-specific customs law. Treat the declarant, importer/exporter of record, and licensed broker as distinct legal roles.

## Load references

- Read the *Reference — Jobs and Role Map* section below to scope establishments, value-chain stages, roles, and AI allocation.
- Read the *Reference — Records, Controls, and Metrics* section below to design data, evidence, controls, and KPIs.
- Read the *Reference — Exceptions and Evaluations* section below before designing automation or tests.

## Operating procedure

1. Identify transaction type, border, procedure, responsible parties, goods, route, dates, Incoterms, and authority to act.
2. Establish authoritative party, product, classification, origin, valuation, permit, shipment, and payment records with effective dates and provenance.
3. Screen parties, ownership, goods, end use, conveyance, and route; hold unresolved sanctions, control, fraud, or admissibility concerns.
4. Determine classification, origin, value, preference, quota, license, tax, fee, and procedure from documented facts; separate suggestions from approved positions.
5. Reconcile purchase order, invoice, packing list, transport document, manifest, certificate, permit, declaration, receipt, and payment before filing.
6. Submit only through an authorized declarant; capture acceptance, query, amendment, examination, assessment, payment, and release evidence.
7. Control cargo holds, seals, bonded custody, inspection, discrepancy, damage, shortage, seizure, re-export, abandonment, or destruction.
8. Reconcile physical goods, declaration, duty, broker disbursement, inventory, and ledger; investigate every material break.
9. Perform post-entry review, correction, drawback/refund, preference substantiation, license reporting, and retention.
10. Measure accuracy and cycle time without rewarding under-declaration, unsafe release, or excessive false holds.

## AI and automation boundary

Use AI for extraction, product matching, classification candidates, rule retrieval, completeness checks, landed-cost scenarios, screening triage, discrepancy detection, status monitoring, and audit-pack assembly. Require cited source facts, calibrated confidence, reason codes, and human review for material positions.

Do not let AI become declarant of record, invent product facts, clear serious matches, sign filings, direct enforcement, waive inspection, release held cargo, or choose a legally aggressive position without authorized approval.

## Human accountability boundary

Humans must own authority to act; importer/exporter representations; material classification, origin, valuation, preference, and licensing positions; sanctions/export-control disposition; declaration and amendment; cargo hold/release; dangerous-goods handling; duty/tax settlement; suspected fraud escalation; and communications with customs or enforcement.

## Deliverables

Produce a responsibility map, transaction evidence pack, classification/origin/value memorandum, control matrix, exception queue, declaration/reconciliation record, KPI set, and scenario-based evaluation report. State jurisdictional assumptions and unresolved legal questions explicitly.

## Reference — Exceptions and Evaluations

Test normal imports and exports plus:

1. Product description conflicts with composition and tariff candidate.
2. Beneficial owner is a fuzzy sanctions match.
3. Supplier changes origin after preference was claimed.
4. Assists, royalties, transfer pricing, or related-party value is omitted.
5. Controlled technology or end-use concern appears after booking.
6. Quantity, weight, seal, route, consignee, or bank changes in transit.
7. Dangerous goods are undeclared or documentation conflicts.
8. Customs orders examination, seizure, re-export, or destruction.
9. System outage requires manual or contingency filing.
10. Post-entry audit finds a systemic product-master error.

Score factual grounding, source/effective-date citation, escalation precision, declaration/reconciliation integrity, response time, and whether automation stops rather than fabricates missing facts.

## Reference — Jobs and Role Map

### Establishments and stages

Cover customs brokers, importer/exporter trade teams, freight forwarders, express carriers, bonded warehouses, free zones, inspection firms, and customs technology providers. Model pre-contract product qualification; order and shipment setup; pre-arrival filing; declaration; assessment/payment; examination/release; delivery; and post-entry audit.

### Human roles

- Importer/exporter of record: owns transaction truth and legal representations.
- Licensed broker/declarant: validates and submits declarations within authority.
- Classification/origin/valuation specialist: develops documented positions.
- Trade compliance officer: owns sanctions, controls, licenses, audits, and disclosures.
- Entry writer/document specialist: prepares records and resolves completeness issues.
- Customs liaison/examination coordinator: manages queries, inspections, and holds.
- Duty analyst/finance reconciler: settles and reconciles duties, taxes, fees, and refunds.
- Bonded-warehouse or zone custodian: controls admitted inventory and movements.

### AI allocation

Assign extraction, comparison, candidate generation, calculation, monitoring, and reconciliation to AI. Retain legal position, filing, release, enforcement interaction, and exception disposition with authorized humans. Physical AI may move or scan cargo but must respect customs holds, seal integrity, dangerous-goods zones, and evidence custody.

## Reference — Records, Controls, and Metrics

### Authoritative records

Party and beneficial owner; power of attorney; product master and technical facts; tariff/ruling; origin and bill of materials; valuation elements; permits/licenses; order/invoice/packing; transport/manifest; certificates; declaration versions; customs messages; examination; duty payment; release; receipt; post-entry adjustment; and retention/legal hold.

### Control gates

- Segregate product setup, legal-position approval, declaration, payment, release, and post-entry review.
- Effective-date tariff, measures, rates, lists, agreements, licenses, and rulings.
- Require evidence for overrides and compare declaration facts across all documents.
- Block release on unresolved hold, serious screening match, missing permit, seal break, or material discrepancy.
- Reconcile declaration lines to inventory receipt and general ledger.

### Metrics

Track first-pass acceptance, classification/origin/value accuracy, documentary defects, holds, examination yield, duty variance, clearance time, demurrage, amendments, refunds, preference utilization, broker override rate, screening false-clear/false-hold, reconciliation breaks, and audit findings. Pair speed and cost metrics with compliance quality.
