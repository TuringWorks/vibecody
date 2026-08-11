---
name: "Operating System 16 — Finance, Insurance, Payments, and Capital Markets"
description: "Operating System 16 — Finance, Insurance, Payments, and Capital Markets: Move money, price risk, allocate capital, protect savings, and enable commerce. Use when the task involves finance, insurance, payments, and capital markets, finance, insurance, payments, capital markets."
category: finance
triggers: ["finance, insurance, payments, and capital markets", "finance", "insurance", "payments", "capital markets"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 16 — Finance, Insurance, Payments, and Capital Markets

> **Layer:** National operating system (#16 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Move money, price risk, allocate capital, protect savings, and enable commerce.

## When to use this skill

Load this skill when a task concerns finance, insurance, payments, and capital markets. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `finance-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When people and firms transact, move money reliably and prevent fraud.
2. When capital is needed, assess risk and allocate funds.
3. When uncertainty exists, insure, hedge, reserve, and regulate.
4. When records matter, account, audit, report, and comply.
5. When consumers need financial help, advise within fiduciary and suitability boundaries.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Banker, loan officer, credit analyst, underwriter.
- Accountant, auditor, controller, financial reporting manager.
- Actuary, risk analyst, compliance analyst, model risk manager.
- Trader, portfolio manager, investment analyst, wealth advisor.
- Claims adjuster, insurance agent, fraud investigator.
- Payments operations analyst, AML analyst, sanctions analyst.
- Fintech product manager, quant researcher, AI risk lead.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Analyst → associate → VP → director → MD (banking); accountant → senior → manager → controller → CFO; actuarial exam ladder; trader/portfolio manager.
- **Skills, tools & tech employers list:** Excel/VBA, Bloomberg/FactSet, SQL/Python, ERP and core-banking, risk systems, AML/KYC platforms (NICE Actimize, World-Check), actuarial software.
- **Qualifications, certifications & licenses:** CPA, CFA, FRM, CAIA, actuarial (ASA/FSA, ACAS/FCAS), CAMS (AML), FINRA Series 7/63/66/24, CFP (advisors).
- **KPIs / metrics in postings:** P&L/return, risk-adjusted metrics (Sharpe, VaR), loss/default and fraud-loss rates, close cycle, regulatory-reporting accuracy, NPS.
- **Where these roles are posted:** eFinancialCareers, LinkedIn, Indeed, Wellfound (fintech), Glassdoor.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `finance-*`. Deploy them under the named human supervisor:

- **KYC/AML review agent** — screens identities and transactions for financial-crime risk. *(supervised by AML analyst; skill: `finance-kyc-aml-review-agent`)*
- **Fraud detection agent** — detects fraud patterns across transactions. *(supervised by fraud investigator; skill: `finance-fraud-detection-agent`)*
- **Credit memo drafter** — drafts credit analyses and memos from financials. *(supervised by credit analyst; skill: `finance-credit-memo-drafter`)*
- **Portfolio research agent** — researches securities and positions. *(supervised by investment analyst; skill: `finance-portfolio-research-agent`)*
- **Insurance claims triage agent** — classifies and routes claims and flags fraud. *(supervised by claims adjuster; skill: `finance-insurance-claims-triage-agent`)*
- **Reconciliation agent** — reconciles ledgers, accounts, and statements. *(supervised by controller; skill: `finance-reconciliation-agent`)*
- **Regulatory reporting assistant** — prepares regulatory filings and disclosures. *(supervised by financial reporting manager; skill: `finance-regulatory-reporting-assistant`)*
- **Financial planning copilot** — models plans within suitability constraints. *(supervised by wealth advisor; skill: `finance-financial-planning-copilot`)*
- **Pricing & ratemaking agent** — develops rate-adequacy analyses, GLM-based pricing, and rate-filing support within actuarial standards of practice. *(supervised by pricing actuary; skill: `finance-pricing-ratemaking-agent`)*
- **Reserving & loss-development agent** — builds loss-development triangles and IBNR estimates (chain-ladder, Bornhuetter-Ferguson) for the reserving actuary. *(supervised by reserving actuary; skill: `finance-reserving-loss-development-agent`)*
- **Actuarial valuation & solvency-reporting agent** — prepares reserves, capital, and disclosures under IFRS 17, Solvency II, and US Stat/RBC for review by the appointed actuary. *(supervised by valuation / appointed actuary; skill: `finance-actuarial-valuation-solvency-reporting-agent`)*
- **Experience-study & mortality agent** — runs experience studies and mortality, morbidity, and lapse assumption analyses. *(supervised by actuary; skill: `finance-experience-study-mortality-agent`)*
- **ALM & economic-capital modeling agent** — models asset-liability matching, economic capital, and stress and scenario results. *(supervised by actuary / risk lead; skill: `finance-alm-economic-capital-modeling-agent`)*

## Humanoid robot roles

- Branch concierge, secure document handling, back-office logistics, facilities support.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Credit denial, fiduciary advice, market conduct, claims disputes, financial-crime escalation, systemic-risk decisions, and the Statement of Actuarial Opinion / appointed-actuary sign-off require human accountability.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Public Finance, Commerce & Services, Governance & Law, Resilience & Continuity. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Energy Abundance](../strategic-missions/energy-abundance/)
- [Strategic Supply Chain](../strategic-missions/strategic-supply-chain/)
- [Science-to-Industry](../strategic-missions/science-to-industry/)
- [Digital Infrastructure](../strategic-missions/digital-infrastructure/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Underwriting and credit judgment and manual modeling erode; traders depend on algorithms.
- **Countermeasures:** Manual underwriting exercises; independent model-risk review; keep judgment in credit and conduct decisions.
- **Role/job simulators (keep-warm):** Underwriting and trading/stress-scenario simulators; manual credit-memo and model builds.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `finance-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
