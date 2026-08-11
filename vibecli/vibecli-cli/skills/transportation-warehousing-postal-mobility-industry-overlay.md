---
name: "Transportation, Warehousing, Postal, and Mobility"
description: "Transportation, Warehousing, Postal, and Mobility: This overlay composes OS 03, 07, 11, 12, 16, 17, and 22. Use when the task involves transportation, warehousing, postal, and mobility, warehousing, postal, mobility."
category: industry
triggers: ["transportation, warehousing, postal, and mobility", "warehousing", "postal", "mobility"]
tools_allowed: ["read_file", "write_file"]
---

# Transportation, Warehousing, Postal, and Mobility

> **Industry ID:** IND-08 · **Accountable human owner:** carrier/terminal executive, licensed dispatcher/controller, safety authority, or fleet operations leader

This overlay composes OS 03, 07, 11, 12, 16, 17, and 22. Read the *Reference — Transport Mode Modifiers* section below for air, rail, maritime, road, warehouse, courier, and passenger modifiers.

## Mission

Move people, goods, mail, and vehicles safely, securely, accessibly, predictably, and economically while preserving custody, condition, capacity, and recovery across networks and modes.

## Core Jobs To Be Done

1. Design networks, schedules, service products, facilities, fleets, capacity, fares/rates, service levels, and resilience.
2. Qualify customers, passengers, cargo, vehicles, operators, routes, dangerous goods, documents, payment, and accessibility needs.
3. Forecast demand, sell/allocate capacity, build loads/manifests, plan crews/equipment, and communicate promises.
4. Accept custody, verify identity/condition/quantity, screen, label, sort, store, stage, load, secure, and document handoffs.
5. Dispatch, route, control, navigate, move, transfer, track, and communicate within weather, traffic, infrastructure, labor, and safety limits.
6. Inspect, fuel/charge, clean, maintain, repair, certify, and release vehicles, equipment, terminals, tracks, vessels, aircraft, and facilities.
7. Deliver or disembark, verify recipient/condition, assist passengers, return equipment, settle charges, and close custody.
8. Detect and recover delay, missed connection, congestion, breakdown, loss, damage, cyber event, severe weather, medical/security event, and capacity shock.
9. Investigate incidents and claims, preserve evidence, compensate fairly, correct causes, and meet reporting duties.
10. Optimize asset utilization, energy, empty movement, working capital, emissions, workforce wellbeing, and long-term network capacity.

## AI and physical-AI allocation

- AI may forecast, price within policy, book, construct loads, schedule, route, slot warehouses, predict maintenance, monitor disruption, prepare documents, communicate status, reconcile charges, and package incident evidence.
- Autonomous trucks, shuttles, trains, port equipment, yard movers, AMRs/forklifts, delivery vehicles/drones, surface vessels, and inspection systems may operate only within approved ODDs, safe-stop behavior, remote assistance, maintenance, and incident logging.
- Safety-critical control authority, air/rail/maritime traffic control, dangerous-goods approval, vehicle release, emergency command, and coercive security remain human/institutional.

## Human accountability boundary

Humans must own safety management and risk acceptance; operator/vehicle certification and release; dangerous-goods acceptance; passenger/cargo denial; emergency and evacuation command; routing through unsafe conditions; hours/fatigue exceptions; accessibility and vulnerable-passenger decisions; security/enforcement escalation; material pricing/refunds/claims; accident findings; public/regulatory reporting; and expansion of any autonomous ODD.

## Systems and controls

Network/schedule/revenue; booking/ticket/order; TMS/dispatch/fleet; WMS/yard/terminal/port/community; manifests/custody/track-and-trace; crew/workforce; maintenance/configuration; maps/weather/traffic/V2X; safety/incident/evidence; dangerous goods/security; billing/freight audit/claims; customer communication; robot/vehicle telemetry/teleoperation.

- Reconcile booking, manifest, physical custody, capacity, movement, delivery, charges, and claims.
- Enforce qualification, hours, route, weather, weight/balance, dangerous-goods, maintenance, access, and ODD gates.
- Separate dispatch/control, maintenance, safety release, billing adjustment, and incident investigation where required.
- Preserve event data, communications, sensor health, map/version, operator/agent action, custody, and override history.

## Metrics and failure modes

Measure safety/near misses, on-time performance, completion, loss/damage, custody defects, load/capacity factor, empty miles, dwell/turn time, warehouse accuracy, first-attempt delivery, maintenance reliability, energy/fuel, cost per movement, claims, accessibility, recovery time, safe stops, disengagements, and remote-assistance load.

Watch for manifest mismatch, unsafe route optimization, fatigue, stale maps, sensor degradation, automation mode confusion, lost custody, inaccessible service, dynamic-price abuse, dangerous-goods misclassification, cascading congestion, teleoperation overload, maintenance deferral, cyber fleet compromise, and correlated autonomous failure.

## Operating procedure

1. Classify mode, service, passenger/cargo, network, custody, jurisdiction, safety regime, and autonomy level.
2. Name carrier, dispatch/control, terminal, maintenance, safety, security, accessibility, customer, and incident owners.
3. Establish authoritative schedule, booking, manifest, asset, operator, map, custody, maintenance, and event records.
4. Allocate planning to AI, certified control to deterministic systems, and physical movement to bounded vehicles/machines.
5. Test weather, congestion, breakdown, lost-link, map error, dangerous goods, medical/security, cyber, evacuation, and manual recovery.
6. Deploy by route/site/ODD with safety case, operational readiness, remote support, incident learning, and fallback capacity.

## Reference — Transport Mode Modifiers

- **Air:** airworthiness, crew duty, weight/balance, slots, weather, ATC, passenger rights, dangerous goods.
- **Rail/transit:** signaling, right of way, platform safety, timetable, control center, accessibility, emergency egress.
- **Maritime/ports:** seaworthiness, pilotage, stowage, stability, tides/weather, port security, pollution, crew welfare.
- **Road/trucking:** driver hours, vehicle condition, route restrictions, weight, roadside safety, last-mile custody.
- **Warehouse/courier/postal:** identity, sort, custody, address quality, prohibited items, loss/damage, delivery proof.
- **Mobility platforms:** driver/operator qualification, pricing, accessibility, safety reports, deactivation appeal, data rights.

Critical exceptions: unmanifested person/cargo, dangerous-goods mismatch, overload, unfit operator, maintenance defect, severe weather, map/signaling error, lost link, medical/security event, missing parcel, custody break, inaccessible service, cyber compromise, infrastructure closure, and mass disruption.
