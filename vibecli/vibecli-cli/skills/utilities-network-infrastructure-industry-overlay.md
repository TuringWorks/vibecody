---
name: "Utilities and Network Infrastructure"
description: "Utilities and Network Infrastructure: This overlay composes OS 06, 07, 11, 12, 19, and 22. Use when the task involves utilities and network infrastructure, utilities, network infrastructure."
category: industry
triggers: ["utilities and network infrastructure", "utilities", "network infrastructure"]
tools_allowed: ["read_file", "write_file"]
---

# Utilities and Network Infrastructure

> **Industry ID:** IND-03 · **Accountable human owner:** system operator, utility executive, control-room authority, or public-health/safety owner

This overlay composes OS 06, 07, 11, 12, 19, and 22. Read the *Reference — Utility Network Modifiers* section below.

## Mission

Continuously deliver safe, reliable, affordable, secure, and sustainable essential network services while balancing flows, protecting public health, restoring failures, and investing ahead of demand.

## Core Jobs To Be Done

1. Forecast demand/supply, plan capacity, site/permit, finance, procure, build, connect, test, and commission network assets.
2. Qualify customers/resources, manage interconnection/service agreements, provision identity/access, meter, rate, bill, collect, and provide assistance/redress.
3. Sense flows, quality, pressure/voltage/frequency/temperature/capacity, asset health, weather, cyber, markets, and public-health/safety conditions.
4. Balance and dispatch generation, storage, pumps, valves, compressors, treatment, traffic/data, demand response, and imports/exports within limits.
5. Inspect, patrol, maintain, calibrate, isolate, repair, replace, and document linear, plant, station, tower, data-center, and customer assets.
6. Detect leak/fault/contamination/overload/outage/cyber event, contain harm, communicate, prioritize critical loads/users, restore, and verify quality.
7. Manage energy/resource markets, procurement, losses, inventories, spares, vegetation/right-of-way, contractors, and mutual aid.
8. Protect operational technology, telemetry, customer data, physical sites, supply chains, and independent safety/protection systems.
9. Report reliability, quality, emissions/discharge, rates, investment, incidents, and customer outcomes to accountable institutions.
10. Exercise black-start/islanding/manual control, degraded communications, emergency allocation, disaster recovery, and long-duration outage plans.

## AI and physical-AI allocation

AI may forecast load/flows, detect anomalies/leaks, optimize pumps/dispatch, predict maintenance, plan restoration, draft permits/customer notices, and simulate contingencies. Drones, crawlers, USVs/AUVs, robots, and autonomous service equipment may inspect and sample bounded assets. Real-time protection, interlocks, process safety, and emergency shutdown remain deterministic and independent.

## Human accountability boundary

Humans must own control-room emergency authority; switching/isolation and worker clearance; nuclear/high-hazard operations; public-health notices; contamination and discharge decisions; load shedding and emergency allocation; customer disconnection; rate/investment policy; major market actions; critical-infrastructure cyber response; restart/re-energization; and regulator/public communication.

## Systems, controls, and metrics

GIS/network model; SCADA/EMS/DMS/BMS/plant control; outage/workforce/asset/maintenance; metering/billing/customer; laboratory/quality; market/trading; weather/forecast; telecom/NOC/data center; OT cyber/identity; permit/environment; emergency/mutual aid; drone/robot telemetry.

Enforce topology, asset identity, telemetry quality, operating limits, switching orders, permits-to-work, protection settings, water/process quality, market limits, customer privacy, and restoration verification. Segregate planning, operation, protection, maintenance clearance, market, billing, and incident review.

Measure availability/reliability, quality, pressure/voltage/frequency, losses, outage customers/minutes, restoration, asset health, preventive maintenance, safety, contamination/spills, cyber incidents, affordability/arrears, emissions/resource intensity, reserve margin, black-start/manual readiness, and autonomous inspection findings.

## Failure modes and operating procedure

Watch for bad telemetry driving control, model/topology error, common-mode automation, hidden customer inequity, protection-model conflict, unsafe remote switching, deferred maintenance, vendor concentration, cyber-physical compromise, alert overload, and loss of manual competence.

1. Classify network, public-health/safety consequence, control hierarchy, market/regulatory model, critical users, and interdependencies.
2. Name system, plant, field safety, public health, cyber, customer, market, environment, and emergency owners.
3. Establish authoritative network, asset, telemetry, setting, work, quality, customer, market, and incident records.
4. Test islanding/black start, contamination/leak, severe weather, communications loss, cyber, protection failure, worker-in-zone, and manual recovery.
5. Deploy analytics/physical AI outside independent protection layers with staged authority, control-room override, drills, and public communication.

## Reference — Utility Network Modifiers

- Electricity: frequency/voltage, reserves, protection, switching, black start, distributed resources.
- Gas/hydrogen/district energy: pressure, odorization/leak, compatibility, combustion, compressor/thermal safety.
- Water/wastewater/irrigation: treatment quality, pressure, contamination, discharge, drought/flood, public health.
- Telecom/data centers: capacity/latency, power/cooling, routing, redundancy, cyber, emergency communications.
- Charging/fuels: connector/fuel quality, payment, queue/capacity, fire, interoperability.

Critical exceptions: telemetry disagreement, protection trip, worker clearance, contamination, leak/fire, low reserve, uncontrolled island, cyber intrusion, communications loss, critical-user outage, market anomaly, severe weather, unsafe restart, and emergency rationing.
