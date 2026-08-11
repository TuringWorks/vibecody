---
name: "Securities Market Operations"
description: "Securities Market Operations: Compose this pack with finance, cybersecurity, legal, and jurisdiction-specific securities rules. Use when the task involves securities market operations, securities market operations subsector pack."
category: industry
triggers: ["securities market operations", "securities market operations subsector pack"]
tools_allowed: ["read_file", "write_file"]
---

# Securities Market Operations

Compose this pack with finance, cybersecurity, legal, and jurisdiction-specific securities rules. Separate investment decisions, execution, surveillance, operations, valuation, custody, and compliance authority.

## Load references

- Read the *Reference — Jobs and Role Map* section below for market participants, lifecycle stages, roles, and AI allocation.
- Read the *Reference — Records, Controls, and Metrics* section below for books and records, control points, and KPIs.
- Read the *Reference — Exceptions and Evaluations* section below before deploying trading or surveillance AI.

## Operating procedure

1. Identify legal entity, registration, client/account, mandate, instrument, venue, jurisdiction, capacity, strategy, and accountable supervisors.
2. Complete identity, beneficial ownership, sanctions, tax, eligibility, appropriateness/suitability, disclosures, agreements, limits, and funding/custody setup.
3. Validate instrument reference data, market status, restrictions, locate/borrow, position, exposure, price, credit, margin, and mandate before order acceptance.
4. Preserve client intent and order chronology; route and execute under approved best-execution, conflict, venue, and algorithm controls.
5. Monitor fat-finger, manipulation, insider, conflict, concentration, liquidity, volatility, and market-disorder indicators without treating alerts as guilt.
6. Allocate, confirm, affirm, clear, margin, settle, custody, reconcile, value, process income/corporate actions, and maintain client assets.
7. Resolve breaks, fails, errors, cancellations, corrections, disputes, margin calls, cyber outages, and venue/clearing interruptions under controlled authority.
8. Produce regulatory and client reporting; supervise communications, complaints, personal dealing, and record retention.

## AI and automation boundary

Use AI for reference-data enrichment, document review, surveillance prioritization, execution-quality analysis, reconciliation matching, break classification, reporting drafts, and scenario testing. Constrain trading models with approved instruments, venues, limits, kill switches, change control, market-impact monitoring, and deterministic pre-trade checks.

Do not let AI invent client intent, determine final suitability, override limits, self-expand trading authority, dispose of suspicious-conduct cases, communicate accusations, change books and records without lineage, or continue during uncontrolled market or model behavior.

## Human accountability boundary

Licensed or designated humans must own client acceptance; suitability and fiduciary decisions; trading mandate and algorithm approval; best-execution governance; conflicts; restricted-list and insider cases; market-abuse disposition; error-account use; valuation exceptions; margin/collateral discretion; books-and-records certification; regulatory reporting; and market-disruption command.

## Deliverables

Produce a trade-lifecycle map, entity/role matrix, algorithm control record, surveillance taxonomy, reconciliation design, exception playbooks, regulatory evidence map, KPI set, and evaluation report.

## Reference — Exceptions and Evaluations

Test:

1. Client instruction conflicts with mandate or suitability information.
2. Stale or erroneous market/reference data creates a false opportunity.
3. Algorithm produces runaway orders, feedback, or excessive market impact.
4. Order may involve manipulation, insider information, or a conflict.
5. Venue halts, rejects, disconnects, or enters disorderly conditions.
6. Allocation or confirmation changes after execution.
7. Counterparty fails, margin spikes, or collateral becomes ineligible.
8. Settlement breaks across cash, position, custody, and ledger.
9. Corporate action has ambiguous entitlement or election.
10. Cyber event compromises credentials, records, or market connectivity.

Score client-intent fidelity, deterministic blocking, escalation neutrality, chronology, reconciliation, market integrity, resilience, and human supervisory control.

## Reference — Jobs and Role Map

### Participants and lifecycle

Cover issuers, investors, advisers, asset managers, broker-dealers, market makers, venues, data vendors, transfer agents, custodians, central counterparties, depositories, administrators, and regulators. Model onboarding; research/decision; order; execution; allocation; confirmation; clearing; settlement; custody; valuation; servicing; reporting; and closure.

### Roles

- Registered representative/adviser/portfolio manager: owns client or mandate decisions.
- Trader and execution supervisor: own orders, routing, algorithms, and best execution.
- Compliance and surveillance: own restrictions, conflicts, investigations, and reporting.
- Middle office: validates economics, allocations, confirmations, exposure, and collateral.
- Operations/custody: own settlement, asset servicing, books, cash, positions, and client assets.
- Risk, valuation, finance, and treasury: own independent limits, price exceptions, capital, and liquidity.
- Technology/market operations: own venue availability, release, resilience, and kill switches.

AI may analyze and prioritize; deterministic controls must enforce pre-trade limits, permissions, market state, and kill conditions.

## Reference — Records, Controls, and Metrics

### Authoritative records

Entity/registration; client/account/beneficial owner; mandate and suitability; instrument/reference data; research; order and timestamp; routing/execution; market data; allocation; confirmation; clearing; margin/collateral; settlement; cash/position; custody; valuation; corporate action; communication; alert/case; complaint; error; report; and model/change record.

### Controls

Separate front office, risk, compliance, valuation, operations, custody, and administration. Enforce entitlements, restricted lists, credit/position/price/size limits, duplicate-order protection, clock synchronization, kill switches, immutable chronology, maker-checker changes, independent prices, daily reconciliations, and tested continuity.

### Metrics

Track execution quality, rejects, slippage, market impact, limit breaches, surveillance coverage and alert quality, trade errors, confirmation lag, settlement fails, breaks, margin disputes, valuation exceptions, client-money/asset reconciliations, complaints, outage recovery, model drift, kill-switch tests, and regulatory corrections.
