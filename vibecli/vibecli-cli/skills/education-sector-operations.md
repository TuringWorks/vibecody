---
name: "Operating System 14 — Education, Training, Libraries, and Human Capital"
description: "Operating System 14 — Education, Training, Libraries, and Human Capital: Form capable people, transmit knowledge, cultivate judgment, and reskill the workforce. Use when the task involves education, training, libraries, and human capital, education, training, libraries, human capital."
category: education
triggers: ["education, training, libraries, and human capital", "education", "training", "libraries", "human capital"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 14 — Education, Training, Libraries, and Human Capital

> **Layer:** National operating system (#14 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Form capable people, transmit knowledge, cultivate judgment, and reskill the workforce.

## When to use this skill

Load this skill when a task concerns education, training, libraries, and human capital. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `education-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When children grow, teach literacy, numeracy, science, citizenship, collaboration, and self-regulation.
2. When workers need new capabilities, assess gaps and train efficiently.
3. When knowledge must persist, preserve, classify, retrieve, and teach it.
4. When learners struggle, adapt instruction and provide support.
5. When credentials matter, assess competence fairly.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Teacher, professor, teaching assistant, tutor, instructional coach.
- Curriculum designer, learning experience designer, assessment specialist.
- School counselor, special education teacher, speech-language pathologist.
- Librarian, archivist, museum educator, knowledge manager.
- Corporate trainer, workforce development specialist, apprenticeship coordinator.
- Education administrator, registrar, student success manager.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Aide/TA → teacher → instructional coach/lead → assistant principal → principal → superintendent; higher ed: adjunct → assistant/associate/full professor; L&D specialist → manager → CLO.
- **Skills, tools & tech employers list:** LMS (Canvas, Schoology), SIS (PowerSchool), assessment platforms, library systems (ILS), instructional-design and EdTech tools.
- **Qualifications, certifications & licenses:** State teaching license/credential (Praxis), subject/special-ed/ESL endorsements, MLS (librarian), administrator credential, ATD/CPTD (L&D).
- **KPIs / metrics in postings:** Learning gains/proficiency, graduation/completion, attendance, credential pass rates, learner satisfaction, time-to-competency.
- **Where these roles are posted:** SchoolSpring, GovernmentJobs (districts), HigherEdJobs, Indeed, LinkedIn, Idealist (nonprofit education).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `education-*`. Deploy them under the named human supervisor:

- **Tutor agent** — diagnoses learner gaps and adapts practice and explanation. *(supervised by teacher; skill: `education-tutor-agent`)*
- **Lesson planner** — drafts standards-aligned lessons and materials. *(supervised by teacher / curriculum designer; skill: `education-lesson-planner`)*
- **Grading assistant** — scores work against rubrics and drafts feedback. *(supervised by teacher; skill: `education-grading-assistant`)*
- **Curriculum alignment checker** — checks materials against standards and outcomes. *(supervised by curriculum designer; skill: `education-curriculum-alignment-checker`)*
- **Knowledge retrieval agent** — finds, classifies, and retrieves knowledge resources. *(supervised by librarian / knowledge manager; skill: `education-knowledge-retrieval-agent`)*
- **Language practice agent** — provides conversational language practice and correction. *(supervised by language teacher; skill: `education-language-practice-agent`)*
- **Career pathway advisor** — maps skills to pathways and training options. *(supervised by student success manager; skill: `education-career-pathway-advisor`)*
- **Accessibility adaptation agent** — adapts materials for accessibility needs. *(supervised by special education teacher; skill: `education-accessibility-adaptation-agent`)*
- **Training simulator** — builds scenario-based practice for skills. *(supervised by corporate trainer; skill: `education-training-simulator`)*

## Humanoid robot roles

- Classroom material support, lab assistant, library shelving/retrieval, campus safety escort.
- Vocational training demonstrator for equipment and procedures.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Child safety, motivation, moral formation, discipline, credentialing, special-needs judgment, and institutional culture need human owners.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Labor & Workforce, Science & Innovation, Culture & Civic Life, Household & Care. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Science-to-Industry](../strategic-missions/science-to-industry/)
- [Frontier Talent Formation](../strategic-missions/frontier-talent-formation/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Teachers lean on AI tutors and lose pedagogy; students offload thinking and lose it too.
- **Countermeasures:** AI as augmentation not replacement; teacher development; assess the process, not just the output.
- **Role/job simulators (keep-warm):** Teaching-practice and classroom-management simulators; lesson-delivery rehearsals; assessment-design drills.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `education-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
