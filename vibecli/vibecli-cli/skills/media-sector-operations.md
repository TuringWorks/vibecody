---
name: "Operating System 18 — Media, Culture, Arts, Sports, Religion, and Civic Life"
description: "Operating System 18 — Media, Culture, Arts, Sports, Religion, and Civic Life: Create meaning, shared narratives, recreation, identity, memory, and social cohesion. Use when the task involves media, culture, arts, sports, religion, and civic life, media, culture, arts, sports."
category: media
triggers: ["media, culture, arts, sports, religion, and civic life", "media", "culture", "arts", "sports", "religion", "civic life"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 18 — Media, Culture, Arts, Sports, Religion, and Civic Life

> **Layer:** National operating system (#18 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Create meaning, shared narratives, recreation, identity, memory, and social cohesion.

## When to use this skill

Load this skill when a task concerns media, culture, arts, sports, religion, and civic life. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `media-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When communities need shared stories, report, create, publish, perform, preserve, and critique.
2. When people need belonging, organize rituals, teams, clubs, events, and civic participation.
3. When misinformation spreads, verify, contextualize, and correct.
4. When cultural assets matter, archive and steward them.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Journalist, editor, producer, fact-checker, documentary researcher.
- Artist, designer, musician, actor, writer, game designer.
- Pastor, chaplain, spiritual care worker, nonprofit program director.
- Coach, athletic trainer, event producer, venue operations manager.
- Archivist, curator, community organizer, communications director.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Assistant/freelancer → reporter/producer/designer → senior/editor → managing editor/creative director; nonprofit: program coordinator → manager → director.
- **Skills, tools & tech employers list:** CMS, Adobe Creative Cloud, NLE (Premiere/Avid), DAM/archive systems, social-publishing and audience-analytics tools.
- **Qualifications, certifications & licenses:** Degrees in journalism/arts (rarely licensed); SAG-AFTRA (performers), seminary/ordination (clergy), coaching certifications, SAA (archivists).
- **KPIs / metrics in postings:** Audience/reach/engagement, subscriptions, accuracy/corrections, event attendance, donations, community trust.
- **Where these roles are posted:** LinkedIn, MediaBistro, JournalismJobs, Idealist (nonprofit), Indeed, guild/industry boards.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `media-*`. Deploy them under the named human supervisor:

- **Research assistant** — gathers and organizes background for stories and projects. *(supervised by journalist / producer; skill: `media-research-assistant`)*
- **Transcript/summarization agent** — transcribes and summarizes interviews and footage. *(supervised by producer; skill: `media-transcript-summarization-agent`)*
- **Localization agent** — localizes content across languages and cultures. *(supervised by communications director; skill: `media-localization-agent`)*
- **Creative drafting assistant** — drafts and iterates creative copy and concepts under human taste. *(supervised by writer / designer; skill: `media-creative-drafting-assistant`)*
- **Audience analytics agent** — analyzes audience engagement and reach. *(supervised by editor; skill: `media-audience-analytics-agent`)*
- **Rights clearance assistant** — tracks rights, licenses, and clearances. *(supervised by producer; skill: `media-rights-clearance-assistant`)*
- **Misinformation monitoring agent** — detects and contextualizes misinformation. *(supervised by fact-checker; skill: `media-misinformation-monitoring-agent`)*

## Humanoid robot roles

- Venue setup, stage logistics, museum-guide support, archive handling, broadcast equipment movement.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Editorial judgment, spiritual authority, artistic taste, community trust, child safeguarding, and live-event responsibility remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Education & Knowledge, Communications & Software, Commerce & Services, Household & Care. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Journalists lose reporting and verification craft; editorial judgment fades.
- **Countermeasures:** Protect reporting fundamentals; verification training; human editorial sign-off.
- **Role/job simulators (keep-warm):** Reporting and verification simulators; misinformation-spotting and editorial-judgment scenarios.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `media-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
