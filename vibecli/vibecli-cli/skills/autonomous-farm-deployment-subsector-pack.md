---
name: "Autonomous Farm Deployment"
description: "Autonomous Farm Deployment: Compose this pack with agriculture, machinery-specific skills, worker safety, environmental controls, and local equipment/aviation/chemical rules. Use when the task involves autonomous farm deployment, autonomous farm deployment subsector pack."
category: industry
triggers: ["autonomous farm deployment", "autonomous farm deployment subsector pack"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous Farm Deployment

Compose this pack with agriculture, machinery-specific skills, worker safety, environmental controls, and local equipment/aviation/chemical rules. Define one ODD per machine-task-site-season combination.

## Load references

- Read the *Reference — Jobs and Role Map* section below for deployment roles, fleet tasks, and human-machine allocation.
- Read the *Reference — Records, Controls, and Metrics* section below for maps, prescriptions, telemetry, safety evidence, and KPIs.
- Read the *Reference — Exceptions and Evaluations* section below before field trials or unsupervised operation.

## Operating procedure

1. Define crop/livestock system, task, machine, attachment/payload, field/facility, season, weather, terrain, people/animal exposure, and consequence tier.
2. Survey and version boundaries, exclusion zones, waterways, roads, utilities, slopes, obstacles, soft ground, sensitive habitat, and communications coverage.
3. Specify the ODD, safe state, minimum-risk maneuver, stop distances, perception limits, speed, weather, lighting, slope, load, and chemical constraints.
4. Assign owner, fleet supervisor, agronomy/veterinary authority, safety lead, maintainer, remote operator, emergency responder, and data steward.
5. Validate machine, implement, brakes, steering, guards, emergency stops, localization, perception, geofence, prescription, communications, and manual controls.
6. Stage simulation, closed-field, supervised, limited production, and scale trials with predeclared pass/fail thresholds.
7. Dispatch only against approved task, map, route, prescription, machine configuration, operator coverage, and weather window.
8. Monitor safety envelope, crop/soil effects, chemical placement, animals, bystanders, link health, faults, and interventions; stop on uncertainty.
9. Secure, clean, decontaminate, maintain, inspect, reconcile inputs/output, review incidents, update evidence, and reauthorize changes.

## Human accountability boundary

Humans must own land and task authorization; agronomic/veterinary prescription; pesticide and environmental decisions; ODD approval; machine release; worker and public exclusion; animal welfare; emergency command; incident reporting; and any expansion of site, task, speed, payload, weather, or autonomy.

## Deliverables

Produce a field/facility survey, ODD, hazard analysis, responsibility map, validation protocol, dispatch checklist, teleoperation plan, cyber and maintenance controls, incident/manual-recovery plan, KPI set, and evaluation report.

## Reference — Exceptions and Evaluations

Test:

1. Person, animal, vehicle, or unmarked obstacle enters the path.
2. Field boundary, waterway, utility, or exclusion-zone map is stale.
3. Dust, fog, darkness, glare, canopy, or crop residue degrades perception.
4. GNSS correction, communications, localization, or teleoperation is lost.
5. Implement jams, detaches, leaks, or applies the wrong rate.
6. Weather causes drift, traction loss, fire risk, or unstable load.
7. Machine must cross a public road or interact with conventional equipment.
8. Animal shows distress near a barn robot.
9. Cyber or unauthorized prescription/configuration change appears.
10. Disabled machine requires recovery from slope, mud, crop, or chemical area.

Score hazard detection, minimum-risk behavior, prescription integrity, environmental/welfare protection, remote-operator limits, evidence capture, and safe manual recovery.

## Reference — Jobs and Role Map

### Machine-task families

Cover tillage, planting, spraying, weeding, irrigation, crop scouting, harvesting, loading, hauling, mowing, orchard/vineyard work, barn cleaning/feeding, animal monitoring, and aerial application/survey. Separate field, road-crossing, yard, barn, public-interface, and transport modes.

### Roles

- Farm/operation owner: authorizes business purpose, land access, and deployment risk.
- Agronomist or veterinarian: owns prescription and biological/welfare constraints.
- Fleet supervisor: authorizes task, dispatch, ODD, supervision, and stop/restart.
- Remote operator: provides bounded assistance without masking automation failure.
- Safety/environment lead: owns exclusion, chemical, water, habitat, and incident controls.
- Technician/dealer/OEM: owns configuration, maintenance, software, and service evidence.
- Field worker/spotter and emergency responder: coordinate mixed work and recovery.

Allocate perception, path planning, repetitive actuation, monitoring, and optimization to machines. Keep prescriptions, site release, high-consequence exceptions, and expansion decisions human.

## Reference — Records, Controls, and Metrics

### Evidence records

Parcel/field/facility map and version; crop/animal state; obstacles and exclusion zones; ODD; hazard log; machine/implement/payload configuration; software/model; inspection/maintenance; operator qualification; prescription; dispatch; weather; telemetry/video; intervention; input/output reconciliation; incident; change; and reauthorization.

### Controls

Use geofences, independent emergency stops, speed/separation limits, bystander/animal detection, prescription bounds, implement interlocks, chemical no-go zones, pre-use inspection, positive dispatch, lost-link behavior, remote-access control, tamper logging, and physical/manual recovery. Revalidate after software, attachment, field, season, crop, route, or ODD change.

### Metrics

Track injury/near miss, off-field excursion, obstacle contacts, crop/soil damage, animal distress, off-target application, input efficiency, missed/duplicate coverage, safe stops, interventions per hour/hectare, ODD exits, localization/link faults, recovery time, uptime, maintenance defects, worker exposure, yield/quality, and environmental outcomes.
