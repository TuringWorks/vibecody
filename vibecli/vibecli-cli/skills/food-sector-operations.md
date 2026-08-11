---
name: "Operating System 05 — Food, Agriculture, Fisheries, and Nutrition"
description: "Operating System 05 — Food, Agriculture, Fisheries, and Nutrition: Produce, inspect, distribute, and stabilize safe food. Use when the task involves food, agriculture, fisheries, and nutrition, food, agriculture, fisheries, nutrition."
category: agriculture
triggers: ["food, agriculture, fisheries, and nutrition", "food", "agriculture", "fisheries", "nutrition"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 05 — Food, Agriculture, Fisheries, and Nutrition

> **Layer:** National operating system (#5 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Produce, inspect, distribute, and stabilize safe food.

## When to use this skill

Load this skill when a task concerns food, agriculture, fisheries, and nutrition. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `food-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When people need calories and nutrition, grow, raise, catch, process, transport, and sell food.
2. When pests, drought, disease, or supply shocks threaten production, adapt quickly.
3. When food moves through supply chains, preserve safety, freshness, labeling, and traceability.
4. When populations face malnutrition or food insecurity, target aid and nutrition programs.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Farmer, ranch manager, farmworker, fishery manager, aquaculture technician.
- Agronomist, soil scientist, crop advisor, irrigation specialist.
- Food scientist, quality assurance manager, food safety inspector.
- Veterinarian, animal health technician, livestock nutritionist.
- Grain merchandiser, cold-chain logistics planner, food distribution manager.
- Dietitian, school nutrition director, food assistance program manager.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Farmworker/technician → crew lead/grower → farm/ranch manager → operations director; agronomy track; food safety: QA tech → QA manager → director of food safety.
- **Skills, tools & tech employers list:** Farm-management software (Climate FieldView, John Deere Operations Center, Granular), precision-ag/GIS, irrigation controllers, telematics, LIMS, HACCP/food-safety systems, ERP.
- **Qualifications, certifications & licenses:** CCA (Certified Crop Adviser), pesticide applicator license, PCQI (FSMA), ServSafe, DVM (veterinary), RD/RDN (dietitian), GlobalG.A.P., CDL for ag transport.
- **KPIs / metrics in postings:** Yield, input cost per acre/unit, loss/waste, food-safety audit scores, traceability completeness, on-time fulfillment.
- **Where these roles are posted:** AgCareers.com, Indeed, LinkedIn, GovernmentJobs (USDA/extension), Snagajob (seasonal/hourly), local co-ops.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `food-*`. Deploy them under the named human supervisor:

- **Crop planning agent** — plans planting, rotation, and inputs against soil, weather, and market data. *(supervised by agronomist; skill: `food-crop-planning-agent`)*
- **Pest/disease detection agent** — detects pests and disease early from imagery and sensor data. *(supervised by crop advisor; skill: `food-pest-disease-detection-agent`)*
- **Weather/yield forecast agent** — forecasts yield and weather risk for planning and hedging. *(supervised by farm manager; skill: `food-weather-yield-forecast-agent`)*
- **Food safety compliance agent** — checks process, labeling, and HACCP records against rules. *(supervised by food safety inspector; skill: `food-food-safety-compliance-agent`)*
- **Traceability analyst** — tracks lots through the supply chain and supports recalls. *(supervised by QA manager; skill: `food-traceability-analyst`)*
- **Commodity market analyst** — analyzes prices, basis, and supply-demand for merchandising. *(supervised by grain merchandiser; skill: `food-commodity-market-analyst`)*
- **Menu nutrition optimizer** — optimizes menus for nutrition, cost, and dietary needs. *(supervised by dietitian; skill: `food-menu-nutrition-optimizer`)*
- **Food assistance eligibility assistant** — screens eligibility and prepares case files for nutrition programs. *(supervised by food assistance program manager; skill: `food-food-assistance-eligibility-assistant`)*
- **Autonomous farm operations agent** — orchestrates the whole farm cycle — plans field tasks, sequences machinery and robots, and tracks progress against the crop plan. *(supervised by farmer / ranch manager; skill: `food-autonomous-farm-operations-agent`)*
- **Irrigation optimization agent** — schedules and meters irrigation against soil moisture, weather, crop stage, and water availability. *(supervised by irrigation specialist; skill: `food-irrigation-optimization-agent`)*
- **Livestock health monitoring agent** — monitors animal health, behavior, and welfare signals and flags issues for the vet. *(supervised by veterinarian / animal health technician; skill: `food-livestock-health-monitoring-agent`)*
- **Autonomous machinery dispatch agent** — dispatches and coordinates tractors, drones, and field robots safely across fields. *(supervised by farm operations manager; skill: `food-autonomous-machinery-dispatch-agent`)*
- **Soil and nutrient optimization agent** — recommends fertilizer, amendments, and variable-rate inputs from soil, tissue, and yield data. *(supervised by agronomist / soil scientist; skill: `food-soil-and-nutrient-optimization-agent`)*
- **Forestry & logging operations agent** — plans sustainable harvest, replanting, and logging logistics within stewardship and permit limits. *(supervised by forester / forestry manager; skill: `food-forestry-logging-operations-agent`)*

## Humanoid robot roles

- Greenhouse work, sorting, packing, harvesting support where crops are robot-suitable.
- Cold-chain warehouse picking, food-service prep support, sanitation.
- Livestock barn inspection assistance under human supervision.

Dedicated **embodied robot role skills** for this sector (LLM-brained; actions as tool calls via VLA policies):

- **Field crop worker robot** — plant, transplant, weed, thin, scout, and selectively hand-harvest row and field crops. *(embodied robot skill: `food-field-crop-worker-robot`)*
- **Orchard and vineyard worker robot** — prune, thin, train, and pick tree fruit, vines, and berries on trellises and canopies. *(embodied robot skill: `food-orchard-and-vineyard-worker-robot`)*
- **Livestock and barn handler robot** — feed, bed, move, and inspect animals and assist milking-prep, weighing, and health checks. *(embodied robot skill: `food-livestock-and-barn-handler-robot`)*
- **Irrigation and field-infrastructure robot** — install, inspect, and repair irrigation, fencing, and field sensors and take soil and tissue samples. *(embodied robot skill: `food-irrigation-and-field-infrastructure-robot`)*

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Autonomous tractor** — till, plant, cultivate, and tow implements across fields to a crop plan with no operator in the seat. *(autonomous machine skill: `food-autonomous-tractor`)*
- **Autonomous harvester / combine** — harvest grain, forage, fruit, or specialty crops and map yield as it goes. *(autonomous machine skill: `food-autonomous-harvester-combine`)*
- **Crop-scouting drone** — fly fields to scout stand, weeds, pests, disease, and irrigation from the air. *(autonomous machine skill: `food-crop-scouting-drone`)*
- **Spraying & seeding drone** — apply crop inputs and seed precisely from the air on a prescription map. *(autonomous machine skill: `food-spraying-seeding-drone`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Accountability boundary”.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Water & Sanitation, Transportation & Logistics, Environment & Waste, Health & Care. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Bioeconomy](../strategic-missions/bioeconomy/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Loss of agronomic and animal-husbandry tacit knowledge; operators cannot farm without precision-ag.
- **Countermeasures:** Extension services; preserve traditional and local knowledge; manual scouting; repairable equipment.
- **Role/job simulators (keep-warm):** Field-scouting and agronomy decision simulators; manual-operation drills on equipment (dual-use with the sector's field world models).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `food-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
