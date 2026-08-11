---
name: "Operating System 03 — Defense, Intelligence, Border, and Foreign Affairs"
description: "Operating System 03 — Defense, Intelligence, Border, and Foreign Affairs: Protect sovereignty, manage alliances, understand threats, control lawful movement, and negotiate with other polities. Use when the task involves defense, intelligence, border, and foreign affairs, defense, intelligence, border, foreign affairs."
category: defense
triggers: ["defense, intelligence, border, and foreign affairs", "defense", "intelligence", "border", "foreign affairs"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 03 — Defense, Intelligence, Border, and Foreign Affairs

> **Layer:** National operating system (#3 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Protect sovereignty, manage alliances, understand threats, control lawful movement, and negotiate with other polities.

## When to use this skill

Load this skill when a task concerns defense, intelligence, border, and foreign affairs. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `defense-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When external threats arise, detect, deter, defend, and recover.
2. When alliances and trade relationships matter, negotiate agreements and preserve channels.
3. When people and goods cross borders, verify identity, safety, legality, and compliance.
4. When adversaries hide intent, gather intelligence and assess risk.
5. When conflict occurs, coordinate logistics, medicine, communications, and rules of engagement.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Diplomat, foreign service officer, consular officer, trade representative.
- Intelligence analyst, OSINT analyst, linguist, threat analyst.
- Soldier, sailor, airman, marine, coast guard, defense planner.
- Border officer, customs specialist, immigration officer, export-control analyst.
- Defense engineer, logistics officer, acquisition manager, cyber operator.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Analyst/officer (entry) → senior analyst → branch chief → SES/flag officer; Foreign Service officer ranks; military O-1…O-6.
- **Skills, tools & tech employers list:** Classified analytic and geospatial (GIS) platforms, OSINT tooling, SIGINT/IMINT systems, language tools, defense logistics systems.
- **Qualifications, certifications & licenses:** TS/SCI clearance (often polygraph), Foreign Service exam, DAWIA (acquisition), language proficiency (DLPT/ILR), military commissioning.
- **KPIs / metrics in postings:** Mission readiness, intelligence timeliness/accuracy, interdiction rates, negotiation/treaty outcomes, force-protection incidents.
- **Where these roles are posted:** USAJOBS, IC Careers (CIA/NSA/DIA/NGA), Feds Hire Vets, ClearanceJobs, agency portals.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `defense-*`. Deploy them under the named human supervisor:

- **OSINT analyst agent** — collects and synthesizes open-source signals into assessed intelligence drafts. *(supervised by intelligence analyst; skill: `defense-osint-analyst-agent`)*
- **Translation agent** — translates and contextualizes multilingual material at speed. *(supervised by linguist / analyst; skill: `defense-translation-agent`)*
- **Sanctions-screening agent** — screens parties and shipments against sanctions and export-control lists. *(supervised by export-control analyst; skill: `defense-sanctions-screening-agent`)*
- **Logistics optimizer** — plans movement of personnel, materiel, and supply under constraints. *(supervised by logistics officer; skill: `defense-logistics-optimizer`)*
- **Red-team simulation agent** — models adversary options and stress-tests plans. *(supervised by defense planner; skill: `defense-red-team-simulation-agent`)*
- **Defense acquisition document reviewer** — reviews requirements, bids, and compliance for acquisition programs. *(supervised by acquisition manager; skill: `defense-defense-acquisition-document-reviewer`)*
- **Intelligence triage agent** — prioritizes and routes incoming reporting and tips. *(supervised by threat analyst; skill: `defense-intelligence-triage-agent`)*
- **Cyber defense agent** — performs continuous monitoring and incident-response assistance. *(supervised by cyber operator; skill: `defense-cyber-defense-agent`)*

## Humanoid robot roles

- Base logistics, warehouse, maintenance, casualty-evacuation support, hazardous-area reconnaissance.
- Border facility support, inspection assistance, disaster-relief unloading.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **ISR reconnaissance drone (UAS)** — conduct intelligence, surveillance, and reconnaissance from the air under human command. *(autonomous machine skill: `defense-isr-reconnaissance-drone-uas`)*
- **Autonomous logistics & resupply vehicle (UGV)** — move materiel, fuel, and casualties across austere terrain without a crewed cab. *(autonomous machine skill: `defense-autonomous-logistics-resupply-vehicle-ugv`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Use of force, detention, asylum determinations, diplomacy, intelligence conclusions, and escalation decisions require human command authority.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Public Safety & Justice, Resilience & Continuity, International Relations, Communications & Software. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Semiconductor Sovereignty](../strategic-missions/semiconductor-sovereignty/)
- [Bioeconomy](../strategic-missions/bioeconomy/)
- [Quantum and Space Systems](../strategic-missions/quantum-and-space-systems/)
- [Strategic Supply Chain](../strategic-missions/strategic-supply-chain/)
- [Cyber Defense](../strategic-missions/cyber-defense/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Over-trust of automated assessments; loss of manual control, analog navigation, and field craft.
- **Countermeasures:** Degraded-comms and manual-reversion drills; maintain analog nav/comms skills; red-teaming.
- **Role/job simulators (keep-warm):** Wargaming and mission simulators; GPS/comms-denied and analog-fallback exercises.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `defense-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
