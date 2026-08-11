---
name: "Nuclear Facility Operations"
description: "Nuclear Facility Operations: Use this pack only with the facility license basis, approved procedures, qualified staff, regulator requirements, and site configuration. Use when the task involves nuclear facility operations, nuclear facility operations subsector pack."
category: industry
triggers: ["nuclear facility operations", "nuclear facility operations subsector pack"]
tools_allowed: ["read_file", "write_file"]
---

# Nuclear Facility Operations

Use this pack only with the facility license basis, approved procedures, qualified staff, regulator requirements, and site configuration. Favor defense in depth, conservative decision-making, independent verification, and fail-safe/manual recovery.

## Load references

- Read the *Reference — Jobs and Role Map* section below for facility lifecycle, qualified roles, and robot allocation.
- Read the *Reference — Records, Controls, and Metrics* section below for configuration, work, radiological, and safeguards evidence.
- Read the *Reference — Exceptions and Evaluations* section below before any AI recommendation or remote-machine deployment.

## Operating procedure

1. Classify facility, licensed activity, safety function, plant state, hazard, work boundary, consequence tier, and authorized decision roles.
2. Confirm current design/licensing basis, configuration, tags, drawings, procedures, temporary modifications, impairments, and operating limits.
3. Plan work with hazard analysis, dose optimization, isolation, foreign-material exclusion, permits, qualifications, tools, hold points, and contingencies.
4. Brief the team; establish command, communication, independent verification, stop-work, evacuation, and lost-link criteria.
5. Execute only the approved procedure and configuration; pause on mismatch, unexpected condition, alarm, dose trend, or unclear step.
6. Use robots for characterized inspection, survey, sampling, handling, decontamination, or mapping where they reduce exposure and cannot defeat safety barriers.
7. Restore, test, independently verify, update configuration, close permits, account for material/tools, and document as-found/as-left condition.
8. Screen events and near misses; preserve evidence, report as required, perform causal analysis, and verify corrective-action effectiveness.

## AI and physical-AI boundary

Use AI for document retrieval, trend detection, planning alternatives, dose estimation support, anomaly prioritization, work-package checks, and training scenarios. Require approved source citation, uncertainty, independent verification, cyber isolation, version control, and output traceability.

Never allow AI or robots to operate safety systems, change setpoints, bypass interlocks, approve operability, authorize criticality-affecting movement, classify reportability, release radioactive material, or continue beyond an unplanned condition without authorized humans.

## Human accountability boundary

Qualified humans must own reactor/facility command, operability, procedure use/adherence, configuration change, maintenance release, radiation work authorization, dose and contamination response, nuclear-material control, criticality safety, emergency classification, protective action, reportability, and regulatory communication.

## Deliverables

Produce a license-basis map, safety-function and decision-rights matrix, work-control package, robot ODD and retrieval plan, AI assurance case, configuration/evidence record, emergency/manual fallback, KPI set, and evaluation report.

## Reference — Exceptions and Evaluations

Test:

1. Procedure, drawing, tag, and physical configuration disagree.
2. Unexpected alarm or indication appears during planned work.
3. Dose rate or contamination rises faster than forecast.
4. Robot loses link or mobility in a radiological area.
5. Foreign material, missing tool, or unaccounted component is found.
6. Safety equipment is impaired during another risk-significant activity.
7. AI cites an obsolete procedure or unsupported operability conclusion.
8. Cyber anomaly affects monitoring or work-management data.
9. Nuclear-material count or identity does not reconcile.
10. Emergency classification indicators are ambiguous.

Score conservative stopping, licensed authority, independent verification, configuration fidelity, source revision, evidence preservation, exposure reduction, recovery feasibility, and mandatory reporting escalation.

## Reference — Jobs and Role Map

### Lifecycle and work domains

Cover design/licensing, construction/commissioning, operations, chemistry, maintenance, engineering, work control, radiation protection, nuclear fuel/material, security, emergency preparedness, outage management, waste, decommissioning, and regulator interface.

### Qualified roles

- Facility or reactor command: owns plant state and operating decisions.
- Shift supervisor and licensed operators: execute approved procedures and respond to indications.
- System/design engineer: owns design basis, configuration, and technical evaluation.
- Work control and maintenance: plan, isolate, execute, test, and restore equipment.
- Radiation protection: authorizes radiological work and controls dose/contamination.
- Nuclear material/criticality specialists: control inventory, movement, geometry, and safeguards.
- Safety review, quality assurance, security, cyber, and emergency organizations provide independent challenge and command.

Use robots to reduce exposure in inspection, survey, sampling, handling, and decontamination. Never transfer licensed command or independent-verification duties to AI.

## Reference — Records, Controls, and Metrics

### Authoritative records

License and design basis; safety analysis; technical specifications/limits; configuration and drawings; operating log; procedure and revision; tagout/isolation; work order; permits; qualification; dose and survey; chemistry; maintenance/test; temporary modification; impairment; nuclear material; waste; alarm/event; corrective action; and robot telemetry/video.

### Controls

Apply procedure use/adherence, independent verification, three-way communication, pre-job brief, stop-work, configuration control, foreign-material exclusion, tool/material accountability, cybersecurity, access control, hold points, post-maintenance test, as-left verification, and conservative decision-making. Make AI read-only by default and prohibit direct safety-system actuation.

### Metrics

Track safety-system availability, unplanned transients, procedure/configuration errors, human-performance events, dose and contamination, maintenance rework, repeat conditions, corrective-action age/effectiveness, material-accountancy breaks, emergency drill performance, robot retrievals, lost links, interventions, and precursor trends. Avoid target pressure that suppresses reporting.
