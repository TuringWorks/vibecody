---
name: "Operating System 06 — Water, Sanitation, and Public Hygiene"
description: "Operating System 06 — Water, Sanitation, and Public Hygiene: Provide safe water, remove waste, control flooding, and prevent waterborne disease. Use when the task involves water, sanitation, and public hygiene, water, sanitation, public hygiene."
category: water
triggers: ["water, sanitation, and public hygiene", "water", "sanitation", "public hygiene"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 06 — Water, Sanitation, and Public Hygiene

> **Layer:** National operating system (#6 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Provide safe water, remove waste, control flooding, and prevent waterborne disease.

## When to use this skill

Load this skill when a task concerns water, sanitation, and public hygiene. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `water-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When people need water, collect, treat, distribute, meter, and maintain supply.
2. When wastewater is produced, collect, treat, discharge, reuse, or recover resources safely.
3. When storms occur, manage drainage and flood protection.
4. When contamination is suspected, test, notify, isolate, and remediate.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Water treatment operator, wastewater operator, utility technician.
- Civil/environmental engineer, hydrologist, water resource planner.
- Plumber, pipefitter, leak detection technician, meter technician.
- Public health inspector, laboratory technician, environmental compliance specialist.
- Floodplain manager, stormwater program manager.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Operator trainee → certified operator (Grade I–IV) → chief operator/superintendent → utility director; engineering: EIT → PE.
- **Skills, tools & tech employers list:** SCADA, GIS, hydraulic modeling (EPANET, WaterGEMS), LIMS, CMMS (asset/maintenance), telemetry.
- **Qualifications, certifications & licenses:** State water/wastewater operator certification (Grades I–IV), PE (civil/environmental), backflow tester, confined-space, CDL (some).
- **KPIs / metrics in postings:** Water-quality compliance, non-revenue water/leakage, NPDES permit compliance, boil-water/outage events, asset condition.
- **Where these roles are posted:** GovernmentJobs, Careers.<state>.gov, AWWA/WEF job boards, Indeed, ZipRecruiter.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `water-*`. Deploy them under the named human supervisor:

- **Water quality monitoring agent** — monitors sensor and lab data and flags contamination signals. *(supervised by treatment operator; skill: `water-water-quality-monitoring-agent`)*
- **Leak prediction agent** — predicts leaks and pipe failures from pressure and acoustic data. *(supervised by utility engineer; skill: `water-leak-prediction-agent`)*
- **Pump optimization agent** — optimizes pumping and energy use across the network. *(supervised by operations engineer; skill: `water-pump-optimization-agent`)*
- **Permit compliance reviewer** — checks discharge and abstraction against permit limits. *(supervised by environmental compliance specialist; skill: `water-permit-compliance-reviewer`)*
- **Flood forecast analyst** — forecasts flood risk and informs drainage operations. *(supervised by floodplain manager; skill: `water-flood-forecast-analyst`)*
- **Asset maintenance planner** — schedules inspection and renewal of network assets. *(supervised by asset manager; skill: `water-asset-maintenance-planner`)*

## Humanoid robot roles

- Plant rounds, valve turning, sample transport, confined-space inspection support with proper safety design.
- Pipe repair assistant, meter reading, emergency sandbag/logistics support.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Water-asset inspection drone** — inspect tanks, towers, pipelines, and treatment assets from the air. *(autonomous machine skill: `water-water-asset-inspection-drone`)*
- **Reservoir survey & sampling vessel (USV)** — survey reservoirs and waterways and collect water-quality samples autonomously. *(autonomous machine skill: `water-reservoir-survey-sampling-vessel-usv`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Public health notices, water shutoffs, infrastructure investment, environmental-discharge approvals, and emergency allocation remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Energy & Utilities, Health & Care, Environment & Waste, Shelter & Built Environment. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Operators cannot run the plant manually during a SCADA failure; process intuition fades.
- **Countermeasures:** Manual-operation drills; operator recertification; contamination tabletops.
- **Role/job simulators (keep-warm):** Plant-operation simulators (SCADA-down); contamination-response and manual-valving drills.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `water-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
