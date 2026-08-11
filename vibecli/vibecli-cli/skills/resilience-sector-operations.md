---
name: "Operating System 22 — Resilience, Continuity, and Strategic Foresight"
description: "Operating System 22 — Resilience, Continuity, and Strategic Foresight: Keep the country functioning through shocks and long-range change. Use when the task involves resilience, continuity, and strategic foresight, resilience, continuity, strategic foresight."
category: resilience
triggers: ["resilience, continuity, and strategic foresight", "resilience", "continuity", "strategic foresight"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 22 — Resilience, Continuity, and Strategic Foresight

> **Layer:** National operating system (#22 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Keep the country functioning through shocks and long-range change.

## When to use this skill

Load this skill when a task concerns resilience, continuity, and strategic foresight. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `resilience-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When risks accumulate slowly, identify weak signals and prepare before failure.
2. When shocks hit, maintain continuity of government, food, water, energy, health, finance, communications, and logistics.
3. When recovery begins, coordinate claims, rebuilding, mental health, supply chains, and accountability.
4. When future scenarios diverge, stress-test systems and invest in options.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Enterprise risk manager, business continuity manager, emergency planner.
- National security planner, infrastructure resilience analyst, scenario planner.
- Supply chain risk manager, insurance catastrophe modeler.
- Crisis communications lead, recovery program manager, mutual aid coordinator.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Analyst → BCM/risk specialist → manager → director of resilience/BCDR; emergency planner → senior → CEM; supply-chain-risk and catastrophe-modeling tracks.
- **Skills, tools & tech employers list:** BCM platforms (Fusion, Archer), GRC, risk registers, scenario/simulation tools, supply-chain mapping, catastrophe models (Moody's RMS, Verisk), GIS.
- **Qualifications, certifications & licenses:** CBCP/MBCP (DRI), CEM, PMP, FRM, ISO 22301 lead auditor, CISSP (cyber-resilience).
- **KPIs / metrics in postings:** RTO/RPO achievement, exercise/test pass rate, time-to-recover, single-point-of-failure coverage, claims throughput.
- **Where these roles are posted:** LinkedIn, Indeed, DRI/continuity boards, USAJOBS/GovernmentJobs (emergency management), ClearanceJobs.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `resilience-*`. Deploy them under the named human supervisor:

- **Scenario generation agent** — generates and stress-tests future scenarios. *(supervised by scenario planner; skill: `resilience-scenario-generation-agent`)*
- **Dependency mapping agent** — maps cross-system dependencies and single points of failure. *(supervised by infrastructure resilience analyst; skill: `resilience-dependency-mapping-agent`)*
- **Crisis dashboard analyst** — maintains a live cross-sector situational picture. *(supervised by emergency planner; skill: `resilience-crisis-dashboard-analyst`)*
- **Continuity plan reviewer** — reviews and tests business-continuity plans. *(supervised by business continuity manager; skill: `resilience-continuity-plan-reviewer`)*
- **Supply disruption monitor** — monitors supply chains for disruption signals. *(supervised by supply chain risk manager; skill: `resilience-supply-disruption-monitor`)*
- **Claims triage agent** — triages post-disaster claims and aid requests. *(supervised by recovery program manager; skill: `resilience-claims-triage-agent`)*

## Humanoid robot roles

- Emergency warehousing, shelter operations, debris assessment, field logistics, hazardous support.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Political prioritization, emergency powers, scarce-resource allocation, evacuation orders, and recovery justice require human legitimacy.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Public Safety & Justice, Defense & Intelligence, Public Finance, Energy & Utilities. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Energy Abundance](../strategic-missions/energy-abundance/)
- [Strategic Supply Chain](../strategic-missions/strategic-supply-chain/)
- [Cyber Defense](../strategic-missions/cyber-defense/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** The meta-owner — continuity planning and the fallback bench themselves can deskill.
- **Countermeasures:** Owns the cross-cutting program: fallback-readiness drills and metrics across all 21 other operating systems.
- **Role/job simulators (keep-warm):** Cross-sector tabletop and full-scale continuity exercises; runs the keep-warm program and bench-readiness metrics for every OS.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `resilience-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
