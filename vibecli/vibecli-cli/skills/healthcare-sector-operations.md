---
name: "Operating System 13 — Healthcare, Public Health, and Biomedical Systems"
description: "Operating System 13 — Healthcare, Public Health, and Biomedical Systems: Prevent disease, diagnose and treat illness, rehabilitate people, and support health across populations. Use when the task involves healthcare, public health, and biomedical systems, healthcare, public health, biomedical systems."
category: healthcare
triggers: ["healthcare, public health, and biomedical systems", "healthcare", "public health", "biomedical systems"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 13 — Healthcare, Public Health, and Biomedical Systems

> **Layer:** National operating system (#13 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Prevent disease, diagnose and treat illness, rehabilitate people, and support health across populations.

## When to use this skill

Load this skill when a task concerns healthcare, public health, and biomedical systems. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `healthcare-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When people are sick or injured, diagnose, treat, monitor, comfort, and follow up.
2. When disease spreads, surveil, trace, vaccinate, communicate, and coordinate.
3. When medicines and devices are needed, research, test, approve, produce, prescribe, and monitor.
4. When care is fragmented, coordinate records, referrals, coverage, and home support.
5. When resources are scarce, triage ethically and transparently.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Physician, nurse practitioner, physician assistant, nurse, pharmacist.
- Medical assistant, phlebotomist, radiologic technologist, lab technician.
- Therapist, psychologist, social worker, care coordinator.
- Epidemiologist, public health nurse, infection preventionist.
- Hospital administrator, revenue cycle analyst, health informatics specialist.
- Clinical researcher, regulatory affairs specialist, biomedical engineer.
- Home health aide, eldercare worker, rehabilitation aide.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Aide/MA/tech → RN/therapist → charge/lead → nurse manager/director → CNO; physician: resident → attending → chief; public health analyst → epidemiologist → health officer.
- **Skills, tools & tech employers list:** EHR (Epic, Cerner), PACS (imaging), CPOE, telehealth, LIS, scheduling, claims/revenue-cycle, disease-surveillance systems.
- **Qualifications, certifications & licenses:** State RN license (NCLEX-RN) with BLS/ACLS/PALS (AHA); MD/DO + board certification + state license + DEA; PA-C/NP; RPh (pharmacist); ARRT (radiology); MPH/CPH (public health); specialty certs (e.g. CCRN).
- **KPIs / metrics in postings:** Clinical quality/outcomes (HCAHPS, readmissions), patient-safety events, length of stay, throughput, coding accuracy, vaccination/coverage rates.
- **Where these roles are posted:** Indeed, Vivian and Incredible Health (nursing), Health eCareers, LinkedIn, GovernmentJobs (public health), hospital career pages.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `healthcare-*`. Deploy them under the named human supervisor:

- **Clinical documentation agent** — drafts notes and structured records from encounters. *(supervised by physician / nurse; skill: `healthcare-clinical-documentation-agent`)*
- **Prior authorization agent** — prepares and submits prior-authorization requests. *(supervised by care coordinator; skill: `healthcare-prior-authorization-agent`)*
- **Care gap analyst** — identifies overdue screenings and care gaps in panels. *(supervised by population health lead; skill: `healthcare-care-gap-analyst`)*
- **Diagnostic support agent** — surfaces differential diagnoses and relevant evidence. *(supervised by physician; skill: `healthcare-diagnostic-support-agent`)*
- **Imaging triage assistant** — prioritizes and pre-reads imaging studies. *(supervised by radiologist; skill: `healthcare-imaging-triage-assistant`)*
- **Drug interaction checker** — checks medication safety and interactions. *(supervised by pharmacist; skill: `healthcare-drug-interaction-checker`)*
- **Public health surveillance agent** — monitors signals for outbreak detection. *(supervised by epidemiologist; skill: `healthcare-public-health-surveillance-agent`)*
- **Outbreak modeler** — models disease spread and intervention scenarios. *(supervised by epidemiologist; skill: `healthcare-outbreak-modeler`)*
- **Clinical trial matching agent** — matches patients to eligible trials. *(supervised by clinical researcher; skill: `healthcare-clinical-trial-matching-agent`)*

## Humanoid robot roles

- Supply delivery, room turnover, lifting support, medication transport, lab sample movement.
- Elder support: fetch, remind, monitor, help with mobility under care-team oversight.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Medical & lab-sample delivery drone** — fly blood, samples, vaccines, and medicines between sites quickly. *(autonomous machine skill: `healthcare-medical-lab-sample-delivery-drone`)*
- **Autonomous supply & pharmacy transport vehicle** — move supplies, meds, linens, and lab samples through a hospital. *(autonomous machine skill: `healthcare-autonomous-supply-pharmacy-transport-vehicle`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Diagnosis, prescribing, surgery, consent, triage, end-of-life decisions, and patient-relationship accountability remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Science & Innovation, Household & Care, Public Safety & Justice, Communications & Software. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Bioeconomy](../strategic-missions/bioeconomy/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Clinicians lose exam and diagnostic skill; radiologists deskill on routine reads; juniors under-train.
- **Countermeasures:** Periodic unaided diagnosis and reads; simulation; keep clinical reasoning central to training.
- **Role/job simulators (keep-warm):** Standardized-patient and procedure simulators; unaided-read sessions; code-blue and rare-presentation sims.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `healthcare-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
