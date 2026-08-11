---
name: "Operating System 20 — Labor, Workforce Systems, and Organizational Life"
description: "Operating System 20 — Labor, Workforce Systems, and Organizational Life: Match people to work, protect workers, build organizations, and maintain productive cultures. Use when the task involves labor, workforce systems, and organizational life, labor, workforce systems, organizational life."
category: hr
triggers: ["labor, workforce systems, and organizational life", "labor", "workforce systems", "organizational life"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 20 — Labor, Workforce Systems, and Organizational Life

> **Layer:** National operating system (#20 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Match people to work, protect workers, build organizations, and maintain productive cultures.

## When to use this skill

Load this skill when a task concerns labor, workforce systems, and organizational life. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `labor-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When work needs doing, define roles, recruit, assess, hire, onboard, train, manage, pay, and retain.
2. When workers are harmed or exploited, enforce labor standards and provide remedy.
3. When technology changes work, redesign jobs and reskill people.
4. When organizations coordinate, set goals, communicate, resolve conflict, and maintain culture.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Recruiter, talent acquisition partner, sourcer, HR business partner.
- Compensation analyst, benefits administrator, payroll specialist.
- Learning and development manager, organizational development consultant.
- Labor relations specialist, employment lawyer, workplace investigator.
- Chief people officer, operations chief, change manager.
- AI workforce transformation lead, automation program manager.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** HR coordinator → HR generalist/recruiter → HR manager/HRBP → director → CHRO; comp, L&D, and employee-relations tracks.
- **Skills, tools & tech employers list:** ATS (Workday, Greenhouse), HRIS, payroll, LMS, people-analytics, compensation-benchmarking and engagement-survey tools.
- **Qualifications, certifications & licenses:** SHRM-CP/SCP, PHR/SPHR (HRCI), CCP (compensation), CEBS (benefits), CPP (payroll), JD (employment law).
- **KPIs / metrics in postings:** Time-to-fill, quality of hire, retention/turnover, engagement (eNPS), pay equity, training completion, compliance.
- **Where these roles are posted:** LinkedIn, Indeed, SHRM, ZipRecruiter, Glassdoor.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `labor-*`. Deploy them under the named human supervisor:

- **Job description agent** — drafts and calibrates job descriptions and scorecards. *(supervised by HR business partner; skill: `labor-job-description-agent`)*
- **Candidate matching assistant** — screens and matches candidates to roles. *(supervised by recruiter; skill: `labor-candidate-matching-assistant`)*
- **Interview scheduling agent** — coordinates interviews and logistics. *(supervised by recruiting coordinator; skill: `labor-interview-scheduling-agent`)*
- **Skills inference agent** — infers skills and gaps from work and history. *(supervised by L&D manager; skill: `labor-skills-inference-agent`)*
- **Training recommender** — recommends learning paths to close gaps. *(supervised by L&D manager; skill: `labor-training-recommender`)*
- **HR policy assistant** — answers policy questions and drafts policy. *(supervised by HR business partner; skill: `labor-hr-policy-assistant`)*
- **Workforce planning simulator** — models headcount, skills, and automation scenarios. *(supervised by workforce planning lead; skill: `labor-workforce-planning-simulator`)*
- **Employee sentiment analyst** — analyzes engagement and sentiment signals. *(supervised by people analytics lead; skill: `labor-employee-sentiment-analyst`)*
- **Corporate development & portfolio agent** — supports M&A screening, portfolio strategy, and corporate-management decisions for holding companies and enterprises. *(supervised by corporate development lead; skill: `labor-corporate-development-portfolio-agent`)*

## Humanoid robot roles

- Workplace facilities support, training-simulation companion, physical-task augmentation.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Hiring decisions, firing, discipline, pay equity, union negotiation, harassment investigations, and culture leadership remain human-accountable.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Education & Knowledge, Governance & Law, Commerce & Services, Manufacturing. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Frontier AI Production](../strategic-missions/frontier-ai-production/)
- [Frontier Talent Formation](../strategic-missions/frontier-talent-formation/)
- [Advanced Manufacturing](../strategic-missions/advanced-manufacturing/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Recruiters and managers lose interviewing and people-judgment skills.
- **Countermeasures:** Keep human judgment in hiring and reviews; manager development.
- **Role/job simulators (keep-warm):** Interview and difficult-conversation role-play simulators; calibration exercises.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `labor-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
