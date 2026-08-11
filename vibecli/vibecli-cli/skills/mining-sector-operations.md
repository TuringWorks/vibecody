---
name: "Operating System 08 — Mining, Materials, Chemicals, and Industrial Inputs"
description: "Operating System 08 — Mining, Materials, Chemicals, and Industrial Inputs: Extract and transform raw materials into safe, reliable inputs for the economy. Use when the task involves mining, materials, chemicals, and industrial inputs, mining, materials, chemicals, industrial inputs."
category: mining
triggers: ["mining, materials, chemicals, and industrial inputs", "mining", "materials", "chemicals", "industrial inputs"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 08 — Mining, Materials, Chemicals, and Industrial Inputs

> **Layer:** National operating system (#8 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Extract and transform raw materials into safe, reliable inputs for the economy.

## When to use this skill

Load this skill when a task concerns mining, materials, chemicals, and industrial inputs. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `mining-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When industry needs inputs, locate, extract, process, refine, transport, and certify materials.
2. When hazardous processes operate, monitor safety and environmental compliance.
3. When supply chains are fragile, diversify sources and recycle critical materials.
4. When materials fail, investigate defects and improve specifications.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Mining engineer, geologist, equipment operator, mine safety manager.
- Chemical engineer, process engineer, plant operator, refinery technician.
- Metallurgist, materials scientist, quality engineer, lab technician.
- Environmental health and safety manager, hazardous materials specialist.
- Supply chain analyst, critical minerals strategist, recycling operations manager.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Operator/technician → process/plant engineer → superintendent → plant manager; geologist and metallurgist tracks.
- **Skills, tools & tech employers list:** DCS process control, LIMS, mine-planning (Surpac, Vulcan), SCADA, EHS systems, simulation.
- **Qualifications, certifications & licenses:** PE, MSHA training, CSP (safety), HAZWOPER, Professional Geologist (PG), CIH (industrial hygiene).
- **KPIs / metrics in postings:** Throughput/recovery, yield and quality, safety (TRIR), environmental compliance, downtime.
- **Where these roles are posted:** Indeed, LinkedIn, ZipRecruiter, mining/chemical industry boards.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `mining-*`. Deploy them under the named human supervisor:

- **Exploration data analyst** — interprets geological and geophysical data to locate resources. *(supervised by geologist; skill: `mining-exploration-data-analyst`)*
- **Process optimization agent** — optimizes yield, energy, and quality in process plants. *(supervised by process engineer; skill: `mining-process-optimization-agent`)*
- **Safety incident predictor** — predicts safety incidents from operations and near-miss data. *(supervised by mine/EHS safety manager; skill: `mining-safety-incident-predictor`)*
- **Chemical literature synthesis agent** — synthesizes chemistry literature and patents for R&D. *(supervised by chemical engineer; skill: `mining-chemical-literature-synthesis-agent`)*
- **Materials discovery agent** — screens and proposes candidate materials and formulations. *(supervised by materials scientist; skill: `mining-materials-discovery-agent`)*
- **Compliance agent** — tracks environmental and safety compliance obligations. *(supervised by EHS manager; skill: `mining-compliance-agent`)*

## Humanoid robot roles

- Hazardous inspection, sample handling, lab/plant logistics, maintenance support.
- Disaster inspection where human entry is dangerous.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Autonomous haul truck** — haul ore and overburden on mine haul roads around the clock. *(autonomous machine skill: `mining-autonomous-haul-truck`)*
- **Autonomous loader / excavator** — load trucks and dig and move material at the face. *(autonomous machine skill: `mining-autonomous-loader-excavator`)*
- **Autonomous blast-hole drill** — drill blast-holes to a pattern precisely and repeatably. *(autonomous machine skill: `mining-autonomous-blast-hole-drill`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Mine safety, hazardous releases, environmental permits, community consent, and shutdown decisions remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Energy & Utilities, Manufacturing, Environment & Waste, Transportation & Logistics. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Energy Abundance](../strategic-missions/energy-abundance/)
- [Semiconductor Sovereignty](../strategic-missions/semiconductor-sovereignty/)
- [Bioeconomy](../strategic-missions/bioeconomy/)
- [Frontier AI Production](../strategic-missions/frontier-ai-production/)
- [Quantum and Space Systems](../strategic-missions/quantum-and-space-systems/)
- [Strategic Supply Chain](../strategic-missions/strategic-supply-chain/)
- [Advanced Manufacturing](../strategic-missions/advanced-manufacturing/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Hazardous-process operators lose hands-on control; geological and metallurgical intuition fades.
- **Countermeasures:** Manual-control drills; hazard simulations; retain deep process knowledge.
- **Role/job simulators (keep-warm):** Process-control and emergency-shutdown simulators; hazard and release-response drills.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `mining-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
