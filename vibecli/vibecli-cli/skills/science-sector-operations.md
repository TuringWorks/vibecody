---
name: "Operating System 15 — Science, Research, Standards, and Innovation"
description: "Operating System 15 — Science, Research, Standards, and Innovation: Discover truth, invent capabilities, validate claims, and turn knowledge into useful systems. Use when the task involves science, research, standards, and innovation, science, research, standards, innovation."
category: science
triggers: ["science, research, standards, and innovation", "science", "research", "standards", "innovation"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 15 — Science, Research, Standards, and Innovation

> **Layer:** National operating system (#15 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Discover truth, invent capabilities, validate claims, and turn knowledge into useful systems.

## When to use this skill

Load this skill when a task concerns science, research, standards, and innovation. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `science-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When unknowns block progress, design experiments and build evidence.
2. When discoveries emerge, replicate, peer review, publish, patent, standardize, and commercialize.
3. When measurement matters, maintain standards, metrology, labs, and reference systems.
4. When research may harm, govern ethics and dual-use risks.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Research scientist, principal investigator, lab manager, research associate.
- Data scientist, computational scientist, statistician, bioinformatician.
- Lab technician, instrumentation specialist, metrologist.
- Grant writer, research administrator, technology transfer officer.
- Patent attorney, standards engineer, regulatory scientist.
- AI researcher, robotics researcher, human factors researcher.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Research associate → scientist → senior/principal investigator → lab/department director; computational and tech-transfer/patent tracks.
- **Skills, tools & tech employers list:** Lab instruments with LIMS/ELN, Python/R, statistical and HPC/simulation software, bioinformatics pipelines, CAD, metrology equipment.
- **Qualifications, certifications & licenses:** PhD (most research-lead roles), PE (standards), USPTO patent bar (patent agent/attorney), GLP/GMP and biosafety training, metrology certifications.
- **KPIs / metrics in postings:** Publications/citations, grants funded, replication/validation rate, patents filed, milestone delivery, measurement accuracy.
- **Where these roles are posted:** Nature Careers, HigherEdJobs, LinkedIn, Indeed, USAJOBS (national labs/NIST), industry R&D pages.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `science-*`. Deploy them under the named human supervisor:

- **Literature review agent** — surveys, synthesizes, and cites the literature. *(supervised by principal investigator; skill: `science-literature-review-agent`)*
- **Hypothesis generator** — proposes testable hypotheses from evidence. *(supervised by research scientist; skill: `science-hypothesis-generator`)*
- **Experiment planner** — designs experiments and power/controls. *(supervised by research scientist; skill: `science-experiment-planner`)*
- **Simulation agent** — runs and analyzes computational simulations. *(supervised by computational scientist; skill: `science-simulation-agent`)*
- **Lab data analyst** — analyzes instrument and assay data. *(supervised by research associate; skill: `science-lab-data-analyst`)*
- **Grant drafting agent** — drafts proposals and budgets. *(supervised by grant writer; skill: `science-grant-drafting-agent`)*
- **Patent landscape analyst** — maps prior art and patent landscapes. *(supervised by technology transfer officer; skill: `science-patent-landscape-analyst`)*
- **Reproducibility checker** — checks methods and data for reproducibility. *(supervised by lab manager; skill: `science-reproducibility-checker`)*
- **Standards comparison agent** — compares methods and results against standards. *(supervised by standards engineer; skill: `science-standards-comparison-agent`)*

## Humanoid robot roles

- Lab automation, sample handling, equipment loading, hazardous-material support.
- Field research support for repetitive measurement and logistics.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Human accountability boundary (must stay human-led)

Research ethics, publication claims, intellectual-property strategy, animal/human-subject decisions, and dual-use release decisions stay human-governed.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Health & Care, Communications & Software, Materials & Manufacturing, Education & Knowledge. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Semiconductor Sovereignty](../strategic-missions/semiconductor-sovereignty/)
- [Bioeconomy](../strategic-missions/bioeconomy/)
- [Frontier AI Production](../strategic-missions/frontier-ai-production/)
- [Quantum and Space Systems](../strategic-missions/quantum-and-space-systems/)
- [Science-to-Industry](../strategic-missions/science-to-industry/)
- [Frontier Talent Formation](../strategic-missions/frontier-talent-formation/)
- [Public Procurement for Frontier Technology](../strategic-missions/public-procurement-for-frontier-technology/)
- [Advanced Manufacturing](../strategic-missions/advanced-manufacturing/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Loss of experimental and statistical craft; over-trust of automated analysis pipelines.
- **Countermeasures:** Reproducibility discipline; manual-analysis competency; train experimental design.
- **Role/job simulators (keep-warm):** Experiment-design and bench-skill simulators; manual analysis and replication exercises; instrument rigs.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `science-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
