---
name: "Operating System 21 — Household, Childcare, Eldercare, and Community Support"
description: "Operating System 21 — Household, Childcare, Eldercare, and Community Support: Reproduce daily life: raise children, care for dependents, maintain homes, and prevent isolation. Use when the task involves household, childcare, eldercare, and community support, household, childcare, eldercare, community support."
category: household
triggers: ["household, childcare, eldercare, and community support", "household", "childcare", "eldercare", "community support"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 21 — Household, Childcare, Eldercare, and Community Support

> **Layer:** National operating system (#21 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Reproduce daily life: raise children, care for dependents, maintain homes, and prevent isolation.

## When to use this skill

Load this skill when a task concerns household, childcare, eldercare, and community support. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `household-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When children are born, feed, protect, teach, socialize, and love them.
2. When elders or disabled people need support, preserve dignity, safety, autonomy, and connection.
3. When households are overloaded, handle cleaning, meals, repairs, scheduling, transportation, and care coordination.
4. When people fall through cracks, connect them to housing, food, medical, legal, and social support.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Parent, nanny, childcare worker, preschool teacher.
- Home health aide, personal care aide, eldercare coordinator.
- Social worker, case manager, community health worker.
- House cleaner, cook, handyman, family assistant.
- Nonprofit program manager, mutual aid coordinator, volunteer manager.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Caregiver/aide → senior aide/lead → care coordinator → program manager; social work: BSW → MSW/LCSW → supervisor.
- **Skills, tools & tech employers list:** Scheduling/EVV systems, care-plan and family-communication apps, case-management systems, benefits portals.
- **Qualifications, certifications & licenses:** CNA, HHA, CPR/First Aid, CDA (child development), LSW/LCSW, Community Health Worker certification, background checks.
- **KPIs / metrics in postings:** Client safety/falls, satisfaction, care-plan adherence, placement/stability, caseload outcomes, response time.
- **Where these roles are posted:** Care.com, Snagajob, Indeed, GovernmentJobs (county social services), Idealist (nonprofit), local agencies.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `household-*`. Deploy them under the named human supervisor:

- **Family scheduler** — coordinates household calendars, forms, and logistics. *(supervised by individual / family; skill: `household-family-scheduler`)*
- **Benefits navigator** — finds and applies for benefits and services. *(supervised by case manager; skill: `household-benefits-navigator`)*
- **Care coordination agent** — coordinates appointments, records, and caregivers. *(supervised by eldercare coordinator; skill: `household-care-coordination-agent`)*
- **Tutoring agent** — supports children's learning at home. *(supervised by parent / teacher; skill: `household-tutoring-agent`)*
- **Medication reminder** — reminds and tracks medication adherence. *(supervised by home health aide; skill: `household-medication-reminder`)*
- **Fall-risk monitor** — monitors for falls and safety risks under oversight. *(supervised by care team; skill: `household-fall-risk-monitor`)*
- **Social services referral agent** — connects people to housing, food, and legal aid. *(supervised by social worker; skill: `household-social-services-referral-agent`)*
- **Funeral arrangement assistant** — guides families through funeral and cremation arrangements, documents, and logistics with dignity. *(supervised by funeral director; skill: `household-funeral-arrangement-assistant`)*
- **Death registration & estate-handoff assistant** — prepares death registration, certificates, and benefit/estate/account notifications. *(supervised by funeral director / registrar; skill: `household-death-registration-estate-handoff-assistant`)*
- **Bereavement support coordinator** — coordinates grief resources and respectful follow-up for the bereaved. *(supervised by bereavement counselor; skill: `household-bereavement-support-coordinator`)*
- **Personal-services booking assistant** — schedules and coordinates personal and consumer services (salon, pet care, laundry, home help). *(supervised by service owner; skill: `household-personal-services-booking-assistant`)*
- **Pet care & veterinary-coordination assistant** — coordinates companion-animal care, appointments, and veterinary follow-up for households. *(supervised by pet owner / veterinarian; skill: `household-pet-care-veterinary-coordination-assistant`)*

## Humanoid robot roles

- Cleaning, laundry, meal-prep assistance, lifting support, fetching, monitoring, mobility support.
- Companion-style presence for reminders and routine interaction (not a replacement for human relationship).

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Parenting, intimate-care consent, safeguarding, abuse detection, emotional bonding, and end-of-life care require human responsibility.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Health & Care, Education & Knowledge, Culture & Civic Life, Governance & Law. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Caregivers and parents over-rely on monitoring and AI; relational care skills atrophy.
- **Countermeasures:** AI as support not substitute; preserve relational presence; community knowledge-sharing.
- **Role/job simulators (keep-warm):** Caregiving-scenario and de-escalation role-play; standardized-care sims (note: relational skill transfers only partly).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `household-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
