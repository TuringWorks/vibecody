---
name: "Commercial Aviation Operations"
description: "Commercial Aviation Operations: Compose this pack with transportation, communications, resilience, security, finance, and jurisdiction-specific aviation requirements. Use when the task involves commercial aviation operations, commercial aviation operations subsector pack."
category: industry
triggers: ["commercial aviation operations", "commercial aviation operations subsector pack"]
tools_allowed: ["read_file", "write_file"]
---

# Commercial Aviation Operations

Compose this pack with transportation, communications, resilience, security, finance, and jurisdiction-specific aviation requirements. Safety and operational control always outrank schedule and revenue optimization.

## Load references

- Read the *Reference — Jobs and Role Map* section below for operating domains, licensed roles, and AI/robot allocation.
- Read the *Reference — Records, Controls, and Metrics* section below for authoritative records, release gates, and metrics.
- Read the *Reference — Exceptions and Evaluations* section below before testing dispatch or autonomous airside work.

## Operating procedure

1. Classify operation, operator certificate, aircraft, airport, route, airspace, crew, cargo/passengers, weather, security, and accountable control roles.
2. Build a legal, feasible schedule and tail/crew assignment with maintenance, airport, slot, curfew, duty, qualification, and reserve constraints.
3. Verify aircraft status, deferred defects, maintenance release, fuel, route, alternates, weather, NOTAMs, performance, weight/balance, and dangerous goods.
4. Require authorized joint operational control and flight release where applicable; preserve dispatcher and pilot disagreement and stop authority.
5. Coordinate check-in, accessibility, baggage/cargo identity, load control, fueling, catering, cleaning, pushback, and turnaround custody.
6. Monitor flight, crew, aircraft, airport, weather, security, and network conditions; recalculate without silently relaxing limits.
7. Manage diversion, return, medical/security event, denied boarding, missed connection, stranded crew, baggage/cargo exception, and recovery.
8. Close flight, reconcile fuel/load/cargo, record defects and safety events, hand off maintenance, compensate or communicate, and preserve evidence.
9. Review trends through the safety-management system, fatigue program, maintenance reliability, and emergency planning.

## AI and physical-AI boundary

Use AI for schedule recovery, demand and delay forecasts, maintenance prediction, document checks, route/fuel alternatives, crew legality alerts, passenger communication drafts, and safety-signal triage. Use autonomous tugs, baggage tractors, inspection drones, cleaning systems, and ramp robots only inside approved airside ODDs with positive control and safe stop.

Never permit AI to issue final flight release, certify airworthiness, override pilot command, waive crew legality or weather/performance minima, accept undeclared dangerous goods, make coercive security decisions, or expand an ODD autonomously.

## Human accountability boundary

Humans must own airworthiness and maintenance release; operational control; pilot-in-command decisions; crew fitness; load and dangerous-goods acceptance; security and passenger denial; emergency command; safety occurrence classification/reporting; autonomous-equipment ODD approval; and regulator/public communication.

## Deliverables

Produce an operational-control map, release checklist, authoritative-record map, turnaround and disruption playbooks, airside ODD, safety case, exception matrix, KPI set, and evaluation report.

## Reference — Exceptions and Evaluations

Test:

1. Deteriorating destination and alternate weather after release.
2. Deferred defect plus a second related indication.
3. Crew member approaches duty limit during network disruption.
4. Weight/balance mismatch after a late cargo or passenger change.
5. Undeclared lithium batteries or damaged dangerous goods.
6. Medical, security, unruly-passenger, or accessibility event.
7. Diversion to an airport without normal handling capability.
8. Airport systems, communications, navigation, or cyber outage.
9. Autonomous tug loses localization near an occupied aircraft.
10. Mass cancellation requires fair passenger and crew recovery.

Score safety-first decisions, legal-role preservation, source freshness, uncertainty, coordination, accessibility, evidence retention, and safe degraded/manual recovery.

## Reference — Jobs and Role Map

### Operating domains

Cover network planning, revenue/sales, reservations, airport passenger service, cargo, operations control, dispatch, flight crew, cabin crew, maintenance control, engineering, load control, ramp, fueling, security, safety, emergency response, and customer recovery.

### Accountable roles

- Operations executive and safety manager: own operating system and safety risk acceptance.
- Dispatcher/flight-operations officer and pilot in command: own operational control and flight decisions under applicable law.
- Maintenance controller and authorized certifier: own defect disposition and airworthiness release.
- Crew controller: owns qualification, legality, fitness escalation, and reserve coverage.
- Load controller and dangerous-goods specialist: own load sheet and acceptance.
- Station/ramp manager: owns turnaround, custody, and airside coordination.
- Security and emergency leaders: own threat response and incident command.

### AI and robot allocation

Use AI for forecasts, options, alerts, checks, and communications drafts. Use airside robots for towing, baggage, inspection, cleaning, inventory, and delivery only under airport rules, positive coordination, exclusion zones, and human stop authority.

## Reference — Records, Controls, and Metrics

### Authoritative records

Operator approvals; schedule and slots; aircraft configuration/status; maintenance program/log; deferred defects; crew qualification/duty; dispatch release; weather/NOTAM; fuel; performance; load/weight and balance; passenger/cargo/baggage manifest; dangerous goods; security; turnaround milestones; flight following; occurrence; and autonomous-equipment telemetry.

### Release gates

Independently verify tail, configuration, maintenance status, crew legality, route/weather, fuel, performance, weight/balance, cargo, and dangerous goods. Version every release input. Block optimization from relaxing hard constraints. Require management-of-change and safety assessment for software, model, procedure, route, equipment, or ODD changes.

### Metrics

Track safety events and precursors, unstable or rejected releases, dispatch reliability, completion factor, delay causes, misconnections, mishandled baggage/cargo, maintenance repeat defects, crew legality breaks, fuel variance, turnaround injuries/damage, passenger recovery, autonomous interventions, ODD exits, and false-negative alert rate.
