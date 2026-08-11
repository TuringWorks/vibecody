---
name: "Operating System 17 — Commerce, Retail, Hospitality, and Customer Operations"
description: "Operating System 17 — Commerce, Retail, Hospitality, and Customer Operations: Match demand to goods and services, create satisfying experiences, and keep commercial operations profitable. Use when the task involves commerce, retail, hospitality, and customer operations, commerce, retail, hospitality, customer operat..."
category: retail
triggers: ["commerce, retail, hospitality, and customer operations", "commerce", "retail", "hospitality", "customer operations"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 17 — Commerce, Retail, Hospitality, and Customer Operations

> **Layer:** National operating system (#17 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Match demand to goods and services, create satisfying experiences, and keep commercial operations profitable.

## When to use this skill

Load this skill when a task concerns commerce, retail, hospitality, and customer operations. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `commerce-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When people want things, discover demand, stock inventory, price, sell, fulfill, support, and retain.
2. When customers need help, understand intent and resolve issues quickly.
3. When services are delivered in person, coordinate labor, space, safety, and experience.
4. When markets change, adapt offerings and channels.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Retail associate, store manager, merchandiser, buyer.
- Account executive, sales development representative, customer success manager.
- Customer support specialist, contact center manager, support operations analyst.
- Hotel front desk manager, housekeeper, concierge, event manager.
- Restaurant manager, chef, line cook, server, food service worker.
- E-commerce manager, marketplace operations manager, growth marketer.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Associate/agent → team lead/shift → store/restaurant manager → district/regional manager → VP ops; sales: SDR → AE → senior AE → sales manager; support: agent → senior → team lead → support manager.
- **Skills, tools & tech employers list:** POS, CRM (Salesforce, HubSpot), e-commerce (Shopify), helpdesk (Zendesk, Intercom), inventory/merchandising, marketing automation.
- **Qualifications, certifications & licenses:** ServSafe (food), TIPS (alcohol service), CHA (hospitality), Salesforce certifications, CCXP (customer experience), OSHA/forklift (backroom).
- **KPIs / metrics in postings:** Sales/conversion, average order value, CSAT/NPS, first-contact resolution, inventory turns, labor cost %, retention/churn.
- **Where these roles are posted:** Snagajob (hourly retail/restaurant), Indeed, ZipRecruiter, LinkedIn (corporate/sales), Wellfound (e-commerce startups).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `commerce-*`. Deploy them under the named human supervisor:

- **Sales research agent** — researches accounts and prospects and qualifies leads. *(supervised by account executive; skill: `commerce-sales-research-agent`)*
- **Proposal generator** — drafts tailored proposals and quotes. *(supervised by account executive; skill: `commerce-proposal-generator`)*
- **Customer support agent** — resolves routine requests and escalates edge cases. *(supervised by support manager; skill: `commerce-customer-support-agent`)*
- **Retention analyst** — predicts churn and recommends retention actions. *(supervised by customer success manager; skill: `commerce-retention-analyst`)*
- **Inventory planning agent** — forecasts demand and plans replenishment. *(supervised by buyer / merchandiser; skill: `commerce-inventory-planning-agent`)*
- **Pricing analyst** — recommends prices and promotions within guardrails. *(supervised by category manager; skill: `commerce-pricing-analyst`)*
- **Review summarizer** — summarizes customer reviews and surfaces issues. *(supervised by product/store manager; skill: `commerce-review-summarizer`)*
- **Marketing campaign agent** — drafts and targets marketing campaigns. *(supervised by growth marketer; skill: `commerce-marketing-campaign-agent`)*
- **Distribution & allocation agent** — coordinates wholesale distribution, allocations, and backorders across the network. *(supervised by distribution operations manager; skill: `commerce-distribution-allocation-agent`)*
- **Wholesale assortment & replenishment agent** — plans wholesale assortment and replenishment against demand and terms. *(supervised by buyer / merchandiser; skill: `commerce-wholesale-assortment-replenishment-agent`)*
- **Equipment-rental fleet & pricing agent** — manages rental/leasing fleet utilization, availability, and pricing. *(supervised by rental operations manager; skill: `commerce-equipment-rental-fleet-pricing-agent`)*
- **Repair-service scheduling & estimate agent** — schedules repair and maintenance jobs and drafts estimates. *(supervised by service manager; skill: `commerce-repair-service-scheduling-estimate-agent`)*

## Humanoid robot roles

- Shelf stocking, room-service delivery, housekeeping support, bussing tables, dish handling.
- Retail floor retrieval, queue assistance, event setup.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Warehouse AMR & autonomous forklift fleet** — move pallets, totes, and racks and feed picking across the facility. *(autonomous machine skill: `commerce-warehouse-amr-autonomous-forklift-fleet`)*
- **Retail inventory & floor-care robot** — scan shelves for stock and pricing and clean floors autonomously after hours. *(autonomous machine skill: `commerce-retail-inventory-floor-care-robot`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Brand trust, customer recovery, labor management, alcohol/regulated sales, safety incidents, and high-value negotiation remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Transportation & Logistics, Finance & Markets, Labor & Workforce, Culture & Civic Life. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Strategic Supply Chain](../strategic-missions/strategic-supply-chain/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Service staff lose customer-recovery craft; managers lose operational intuition.
- **Countermeasures:** Preserve human service and escalation skills; scenario training.
- **Role/job simulators (keep-warm):** Service-recovery and difficult-customer role-play simulators; operations-scenario drills.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `commerce-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
