---
name: "Operating System 04 — Public Safety, Justice Operations, and Emergency Response"
description: "Operating System 04 — Public Safety, Justice Operations, and Emergency Response: Prevent harm, respond to emergencies, maintain order, and recover from acute incidents. Use when the task involves public safety, justice operations, and emergency response, public safety, justice operations, emergency response."
category: public-safety
triggers: ["public safety, justice operations, and emergency response", "public safety", "justice operations", "emergency response"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 04 — Public Safety, Justice Operations, and Emergency Response

> **Layer:** National operating system (#4 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Prevent harm, respond to emergencies, maintain order, and recover from acute incidents.

## When to use this skill

Load this skill when a task concerns public safety, justice operations, and emergency response. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `public-safety-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When someone is in danger, receive the signal, dispatch help, and stabilize the situation.
2. When crime occurs, investigate, preserve evidence, and support prosecution or restorative processes.
3. When fires, floods, earthquakes, pandemics, or industrial accidents occur, coordinate multi-agency response.
4. When infrastructure fails, prioritize rescue, shelter, utilities, medicine, and public communication.
5. When risk can be reduced, inspect, educate, enforce, and prepare.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- 911 dispatcher, emergency manager, incident commander.
- Firefighter, EMT, paramedic, search-and-rescue specialist.
- Police officer, detective, crime analyst, forensic technician.
- Probation officer, corrections officer, victim advocate.
- Safety inspector, fire marshal, disaster recovery specialist.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Recruit/officer/EMT → detective/paramedic/senior → sergeant/lieutenant/captain → chief; dispatcher → comms supervisor; emergency-management coordinator → director.
- **Skills, tools & tech employers list:** CAD (computer-aided dispatch), RMS (records management), NIMS/ICS, body-cam/evidence systems, NCIC, GIS.
- **Qualifications, certifications & licenses:** POST certification (police), state EMT/Paramedic (NREMT), Firefighter I/II, EMD, FEMA ICS/NIMS, CEM (certified emergency manager).
- **KPIs / metrics in postings:** Response and call-answer times, case clearance rate, incident outcomes, mutual-aid readiness, safety.
- **Where these roles are posted:** GovernmentJobs, National Testing Network/PoliceApp, USAJOBS, local agency sites, Snagajob (some support roles).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `public-safety-*`. Deploy them under the named human supervisor:

- **Emergency call triage assistant** — classifies incoming calls, extracts location and severity, and supports dispatch. *(supervised by 911 dispatch supervisor; skill: `public-safety-emergency-call-triage-assistant`)*
- **Dispatch optimizer** — allocates and routes responders against live demand. *(supervised by emergency manager; skill: `public-safety-dispatch-optimizer`)*
- **Incident summarization agent** — maintains a live common operating picture and after-action logs. *(supervised by incident commander; skill: `public-safety-incident-summarization-agent`)*
- **Crime pattern analyst** — detects spatial-temporal crime patterns and links cases. *(supervised by crime analyst; skill: `public-safety-crime-pattern-analyst`)*
- **Evidence chain-of-custody assistant** — tracks evidence handling and flags integrity gaps. *(supervised by forensic technician; skill: `public-safety-evidence-chain-of-custody-assistant`)*
- **Forensic media review agent** — reviews video/audio/digital media for relevant events. *(supervised by detective; skill: `public-safety-forensic-media-review-agent`)*
- **Disaster scenario planner** — models hazard scenarios and resource needs. *(supervised by emergency planner; skill: `public-safety-disaster-scenario-planner`)*
- **Public alert drafting agent** — drafts multilingual, accessible public warnings. *(supervised by public information officer; skill: `public-safety-public-alert-drafting-agent`)*
- **Resource allocation agent** — matches shelters, supplies, and crews to needs. *(supervised by logistics chief; skill: `public-safety-resource-allocation-agent`)*

## Humanoid robot roles

- Hazardous entry, fireground supply movement, stretcher support, debris inspection.
- Shelter logistics, food/water distribution, sanitation support.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Search & response drone** — search for people, map incidents, and deliver overhead situational awareness in emergencies. *(autonomous machine skill: `public-safety-search-response-drone`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Arrests, use of force, triage in scarce life-saving situations, sentencing, detention, and incident command remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Defense & Intelligence, Health & Care, Resilience & Continuity, Water & Sanitation. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Cyber Defense](../strategic-missions/cyber-defense/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Dispatch and response depend on CAD; incident commanders lose improvisation under protocolized tools.
- **Countermeasures:** Manual-dispatch drills; full-scale exercises with technology disabled; sim-based skills currency.
- **Role/job simulators (keep-warm):** Incident-command and dispatch simulators; tech-down field exercises; EMS code-blue sims.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `public-safety-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
