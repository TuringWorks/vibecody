---
name: "Medical & lab-sample delivery drone"
description: "Medical & lab-sample delivery drone: The Medical & lab-sample delivery drone is a non-humanoid autonomous machine whose job is to fly blood, samples, vaccines, and medicines between sites quickly. Use when the task involves medical & lab-sample delivery drone, healthcare."
category: healthcare
triggers: ["medical & lab-sample delivery drone", "healthcare"]
tools_allowed: ["read_file", "write_file"]
---

# Medical & lab-sample delivery drone

> **Operating system:** 13. Healthcare, Public Health, and Biomedical Systems · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** hospital networks, rural clinics, lab-logistics routes
> **Sector skill:** `healthcare-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Medical & lab-sample delivery drone** is a non-humanoid autonomous machine whose job is to fly blood, samples, vaccines, and medicines between sites quickly. BVLOS medical-delivery UAV (a mature use case in several countries); requires airspace authorization; cold-chain and chain-of-custody preserved.

## Operating-system context

This platform serves the *Healthcare* operating system, whose mission is to prevent disease, diagnose and treat illness, rehabilitate people, and support health across populations. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "fly blood, samples, vaccines, and medicines between sites quickly" in environments such as hospital networks, rural clinics, lab-logistics routes. Pair with the sector skill (`healthcare-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `healthcare-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Division of labor and safety”.

## Accountability boundary

Diagnosis, prescribing, surgery, consent, triage, end-of-life decisions, and patient-relationship accountability remain human-led.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Aide/MA/tech → RN/therapist → charge/lead → nurse manager/director → CNO; physician: resident → attending → chief; public health analyst → epidemiologist → health officer.
- **Skills, tools & tech employers list:** EHR (Epic, Cerner), PACS (imaging), CPOE, telehealth, LIS, scheduling, claims/revenue-cycle, disease-surveillance systems.
- **Qualifications, certifications & licenses:** State RN license (NCLEX-RN) with BLS/ACLS/PALS (AHA); MD/DO + board certification + state license + DEA; PA-C/NP; RPh (pharmacist); ARRT (radiology); MPH/CPH (public health); specialty certs (e.g. CCRN).
- **KPIs / metrics in postings:** Clinical quality/outcomes (HCAHPS, readmissions), patient-safety events, length of stay, throughput, coding accuracy, vaccination/coverage rates.
- **Where these roles are posted:** Indeed, Vivian and Incredible Health (nursing), Health eCareers, LinkedIn, GovernmentJobs (public health), hospital career pages.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
