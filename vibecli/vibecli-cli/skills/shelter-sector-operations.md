---
name: "Operating System 10 — Shelter, Construction, Land, and the Built Environment"
description: "Operating System 10 — Shelter, Construction, Land, and the Built Environment: Create and maintain places for living, working, mobility, commerce, and public life. Use when the task involves shelter, construction, land, and the built environment, shelter, construction, land, built environment."
category: construction
triggers: ["shelter, construction, land, and the built environment", "shelter", "construction", "land", "built environment"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 10 — Shelter, Construction, Land, and the Built Environment

> **Layer:** National operating system (#10 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Create and maintain places for living, working, mobility, commerce, and public life.

## When to use this skill

Load this skill when a task concerns shelter, construction, land, and the built environment. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `shelter-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When people need shelter and workspaces, plan, finance, permit, build, inspect, and maintain them.
2. When land is scarce, balance housing, infrastructure, ecology, commerce, and fairness.
3. When buildings age, renovate, retrofit, or demolish safely.
4. When hazards change, improve resilience to heat, fire, flood, wind, and seismic risk.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Urban planner, zoning analyst, real estate developer, housing policy analyst.
- Architect, civil engineer, structural engineer, MEP engineer.
- Construction manager, superintendent, estimator, scheduler.
- Carpenter, electrician, plumber, HVAC technician, mason, roofer.
- Building inspector, code official, facilities manager, property manager.
- Surveyor, GIS analyst, land acquisition specialist.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Laborer/apprentice → journeyman tradesperson → foreman/superintendent → project manager; design: intern architect/EIT → licensed architect/PE → principal; planner → senior planner → director.
- **Skills, tools & tech employers list:** BIM (Revit), AutoCAD, Procore/Bluebeam, estimating (PlanSwift), scheduling (Primavera P6, MS Project), GIS, permitting systems.
- **Qualifications, certifications & licenses:** PE, licensed architect (ARE/AIA), LEED, PMP, OSHA 30, ICC code certifications, trade journeyman/master licenses, PLS (surveyor).
- **KPIs / metrics in postings:** Schedule/cost variance (CPI/SPI), safety (TRIR/EMR), punch-list/defects, inspection pass rate, permit cycle time.
- **Where these roles are posted:** Indeed, LinkedIn, ZipRecruiter, construction boards, GovernmentJobs (inspectors/planners), trade unions.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `shelter-*`. Deploy them under the named human supervisor:

- **Permitting assistant** — guides and pre-checks permit applications against code. *(supervised by code official; skill: `shelter-permitting-assistant`)*
- **Code compliance checker** — checks designs and plans against building codes. *(supervised by building inspector; skill: `shelter-code-compliance-checker`)*
- **Construction scheduler** — builds and maintains critical-path construction schedules. *(supervised by project scheduler; skill: `shelter-construction-scheduler`)*
- **Design option generator** — generates and compares design options against constraints. *(supervised by architect; skill: `shelter-design-option-generator`)*
- **Quantity takeoff estimator** — produces material and cost takeoffs from drawings. *(supervised by estimator; skill: `shelter-quantity-takeoff-estimator`)*
- **Energy modeling agent** — models building energy and comfort performance. *(supervised by MEP engineer; skill: `shelter-energy-modeling-agent`)*
- **Facilities maintenance planner** — plans preventive maintenance across a building portfolio. *(supervised by facilities manager; skill: `shelter-facilities-maintenance-planner`)*
- **Lease/document reviewer** — reviews leases and property documents for terms and risk. *(supervised by property manager; skill: `shelter-lease-document-reviewer`)*
- **Property listing & valuation agent** — drafts listings and runs comparable-based valuations (AVM) for sale or rent. *(supervised by real-estate broker; skill: `shelter-property-listing-valuation-agent`)*
- **Lease abstraction & management agent** — extracts lease terms and tracks obligations, renewals, and escalations. *(supervised by property manager; skill: `shelter-lease-abstraction-management-agent`)*
- **Tenant screening & onboarding assistant** — screens applicants and prepares onboarding within fair-housing and anti-discrimination limits. *(supervised by property manager; skill: `shelter-tenant-screening-onboarding-assistant`)*

## Humanoid robot roles

- Material handling, site cleanup, inspection, painting, drywall support, repetitive tool tasks.
- Facilities rounds, repair support, janitorial work, disaster damage assessment.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Autonomous earthmover (dozer/excavator/loader)** — grade, excavate, load, and move material to a site model. *(autonomous machine skill: `shelter-autonomous-earthmover-dozer-excavator-loader`)*
- **Site survey & progress drone** — map the site, track earthwork volumes, and monitor progress and safety from the air. *(autonomous machine skill: `shelter-site-survey-progress-drone`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Land-use decisions, structural signoff, occupancy approval, worker safety, eviction, and public consultation remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Water & Sanitation, Energy & Utilities, Transportation & Logistics, Environment & Waste. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Energy Abundance](../strategic-missions/energy-abundance/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Trades deskilled by prefab and robotics; inspectors over-rely on AI for structural judgment.
- **Countermeasures:** Apprenticeship protection; manual-inspection competency; retain structural judgment.
- **Role/job simulators (keep-warm):** Inspection and structural-judgment simulators; VR/AR trade-skill rigs; manual quantity-takeoff practice.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `shelter-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
