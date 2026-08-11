---
name: "Operating System 02 — Public Finance, Tax, Treasury, and Procurement"
description: "Operating System 02 — Public Finance, Tax, Treasury, and Procurement: Collect revenue, allocate budgets, buy public goods, manage debt, and protect public money. Use when the task involves public finance, tax, treasury, and procurement, public finance, tax, treasury, procurement."
category: public-finance
triggers: ["public finance, tax, treasury, and procurement", "public finance", "tax", "treasury", "procurement"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 02 — Public Finance, Tax, Treasury, and Procurement

> **Layer:** National operating system (#2 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Collect revenue, allocate budgets, buy public goods, manage debt, and protect public money.

## When to use this skill

Load this skill when a task concerns public finance, tax, treasury, and procurement. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `public-finance-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When public services need funding, collect taxes and fees fairly so the state can operate.
2. When money is limited, prioritize budgets so public value is maximized.
3. When agencies need goods or services, procure transparently so corruption and waste are minimized.
4. When financial risks emerge, forecast cash flow, debt, pensions, and macroeconomic exposure.
5. When public funds are spent, audit and report results so citizens can trust the system.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Tax examiner, revenue agent, tax policy analyst, collections specialist.
- Budget analyst, financial analyst, treasury analyst, grants manager.
- Procurement officer, contract specialist, vendor manager, sourcing analyst.
- Auditor, controller, forensic accountant, inspector general investigator.
- Economist, actuary, fiscal policy advisor, public pension analyst.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Staff accountant/tax examiner → senior analyst/auditor → manager/controller → finance director/CFO; procurement: buyer → contract specialist → warranted contracting officer. Public roles carry GS grades.
- **Skills, tools & tech employers list:** ERP (SAP, Oracle, Workday), GL/AP and tax systems, Excel/Power BI, e-sourcing/procurement (SAP Ariba, Coupa), GASB/GAAP reporting, data-analytics.
- **Qualifications, certifications & licenses:** CPA, CGFM (government financial manager), CIA, CFE (fraud), CPPB/CPPO and FAC-C/DAWIA (federal contracting), CGAP.
- **KPIs / metrics in postings:** Collection rate, days-to-close, budget variance, audit findings, procurement cycle time, savings captured, fraud loss rate.
- **Where these roles are posted:** USAJOBS, GovernmentJobs, LinkedIn, Indeed; AGA/GFOA boards for public finance.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `public-finance-*`. Deploy them under the named human supervisor:

- **Tax return review agent** — screens returns for errors and anomalies and prepares examiner work files. *(supervised by tax examiner / revenue agent; skill: `public-finance-tax-return-review-agent`)*
- **Anomaly detection agent** — flags irregular transactions and patterns across revenue and spending data. *(supervised by controller / auditor; skill: `public-finance-anomaly-detection-agent`)*
- **Audit sampling agent** — selects statistically defensible samples and assembles evidence. *(supervised by auditor; skill: `public-finance-audit-sampling-agent`)*
- **Budget scenario modeler** — models budget tradeoffs, distributional impacts, and multi-year scenarios. *(supervised by budget analyst; skill: `public-finance-budget-scenario-modeler`)*
- **Grant compliance reviewer** — checks grant spending against terms and prepares findings. *(supervised by grants manager; skill: `public-finance-grant-compliance-reviewer`)*
- **Procurement drafting agent** — drafts RFPs, evaluates bids against criteria, and tracks obligations. *(supervised by procurement officer; skill: `public-finance-procurement-drafting-agent`)*
- **Vendor risk analyst** — scores supplier financial, delivery, and integrity risk. *(supervised by vendor manager; skill: `public-finance-vendor-risk-analyst`)*
- **Invoice reconciliation agent** — matches invoices, POs, and receipts and resolves exceptions. *(supervised by accounts-payable lead; skill: `public-finance-invoice-reconciliation-agent`)*
- **Fraud detection agent** — detects procurement and benefits fraud signals for investigation. *(supervised by inspector general investigator; skill: `public-finance-fraud-detection-agent`)*
- **Pension & retirement valuation agent** — performs actuarial pension valuations (funding status, PBO/ABO, contribution projections) for review by the plan actuary. *(supervised by public pension actuary; skill: `public-finance-pension-retirement-valuation-agent`)*

## Humanoid robot roles

- Mailroom, scanning, inventory, warehouse, and records logistics support.
- Physical asset inspection support for public property inventories.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Tax enforcement, budget authority, contract awards, debt issuance, and fraud prosecution remain human/institutional decisions.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Governance & Law, Finance & Markets, Resilience & Continuity. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Science-to-Industry](../strategic-missions/science-to-industry/)
- [Public Procurement for Frontier Technology](../strategic-missions/public-procurement-for-frontier-technology/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Auditors lose forensic judgment; budget and procurement analysts cannot model or evaluate bids unaided.
- **Countermeasures:** Manual audit-sampling exercises; build-from-scratch modeling practice; fraud red-teams.
- **Role/job simulators (keep-warm):** Audit and fraud-investigation simulators on synthetic ledgers; manual budget-model and bid-evaluation builds.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `public-finance-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
