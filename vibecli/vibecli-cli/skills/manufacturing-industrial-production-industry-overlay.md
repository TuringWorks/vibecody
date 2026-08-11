---
name: "Manufacturing and Industrial Production"
description: "Manufacturing and Industrial Production: This overlay composes OS 07, 08, 09, 11, 12, 15, 19, and 20. Use when the task involves manufacturing and industrial production, manufacturing, industrial production."
category: industry
triggers: ["manufacturing and industrial production", "manufacturing", "industrial production"]
tools_allowed: ["read_file", "write_file"]
---

# Manufacturing and Industrial Production

> **Industry ID:** IND-05 · **Accountable human owner:** plant manager, operations executive, quality authority, or product-release owner

This overlay composes OS 07, 08, 09, 11, 12, 15, 19, and 20. Read the *Reference — Production Models and Modifiers* section below for subsector and production-model modifiers.

## Mission

Convert materials, energy, designs, labor, and know-how into conforming products safely, reliably, competitively, traceably, and with responsible lifecycle impacts.

## Core Jobs To Be Done

1. Translate market, customer, regulatory, safety, sustainability, and service needs into controlled product and process requirements.
2. Design product, process, tooling, test, work instructions, capacity, layout, supply, maintenance, and quality controls; validate before release.
3. Qualify suppliers, materials, components, software, tooling, contract manufacturers, and change notifications.
4. Forecast, plan, schedule, kit, stage, and dispatch work across constraints in labor, materials, machines, energy, tools, and due dates.
5. Set up, run, monitor, control, and document conversion/assembly while preserving lot, batch, serial, recipe, revision, and operator genealogy.
6. Inspect, measure, test, sample, quarantine, disposition, release, package, label, store, and ship product with calibrated evidence.
7. Maintain, calibrate, clean, change over, repair, and improve assets without bypassing safety or validated state.
8. Detect deviations, defects, cyber/process anomalies, supplier escapes, and unsafe conditions; contain, investigate, correct, and verify effectiveness.
9. Manage engineering/process/supplier/software changes, substitutions, concessions, rework, scrap, obsolescence, warranty, and field feedback.
10. Trace and recall affected product, notify accountable parties, support customers, recover operations, and update design/process controls.

## AI and physical-AI allocation

- AI may support CAD/CAM, requirements comparison, simulation, scheduling, work instructions, parameter recommendations, anomaly detection, inspection review, predictive maintenance, root cause, supplier risk, documentation, genealogy queries, and recall scoping.
- Robots and autonomous systems may machine, weld, assemble, dispense, inspect, package, palletize, tend equipment, move material, clean, and monitor bounded cells and routes.
- Deterministic PLC/SIS/interlocks retain time-critical control and safe state. Learned systems must not bypass guards, lockout/tagout, recipe limits, validated methods, or release gates.

## Human accountability boundary

Humans must own product/process design authority; validated-state acceptance; safety-critical settings and overrides; hazardous-process authorization; lockout/tagout; supplier approval; material review and nonconformance disposition; quality release; concessions/deviations; regulatory submissions; worker actions; recall; environmental release; and communications to customers, regulators, workers, insurers, or the public.

## Systems and controls

PLM/CAD/BOM; ERP/MRP; MES/electronic batch record; APS/scheduling; QMS/LIMS/SPC; WMS/traceability; EAM/CMMS/calibration; SCADA/historian/OT security; supplier quality; labeling/serialization; warranty/service; environmental/safety; robot/fleet telemetry.

- Enforce approved BOM/recipe/routing/revision and effective dates at issue and execution.
- Segregate design, change approval, production, inspection, release, inventory adjustment, and scrap disposition.
- Reconcile physical product, genealogy, inventory, quality status, and financial records.
- Validate measurement systems, software, models, methods, and robot programs before production use.
- Stop and quarantine on identity, revision, calibration, parameter, guard, quality, or traceability failure.

## Metrics and failure modes

Measure OEE, throughput, schedule attainment, yield, scrap/rework, first-pass quality, defects/escapes, capability, changeover, downtime/MTBF/MTTR, maintenance compliance, supplier quality, inventory/WIP, energy/material intensity, safety/near misses, recall scope/time, cost/unit, and automation intervention/correction.

Watch for wrong revision/material, hidden rework, sensor drift, model-induced process drift, robot collision, unsafe optimization, genealogy gaps, quality inspection trained on defective labels, cyber manipulation, maintenance deferral, correlated fleet failure, and throughput pressure overriding stop-work authority.

## Operating procedure

1. Classify subsector, discrete/batch/continuous model, product risk, regulatory regime, site, process hazards, and customer reliance.
2. Name design, process, plant, quality, safety, maintenance, OT cyber, supply, and release owners.
3. Establish authoritative requirements, BOM/recipe, routing, genealogy, quality, asset, and change records.
4. Allocate cognition to AI, stable control to deterministic systems, and bounded physical execution to validated robots/machines.
5. Test wrong-part/revision, drift, defect, injury, contamination, cyber, power loss, supplier failure, recall, and manual recovery.
6. Deploy through simulation, pilot, process qualification, controlled ramp, audit, incident learning, and human keep-warm drills.

## Reference — Production Models and Modifiers

- **Discrete/assembly:** unit serials, BOM/routing, torque/fit, configuration, software/firmware, end-of-line test.
- **Batch/process:** recipe, material status, potency/concentration, cleaning, contamination, sample plan, batch release.
- **Continuous:** stable control, process safety, transitions, alarm management, custody/quantity, shutdown/startup.
- **Food/pharma/medical:** hygiene, allergens/sterility, validation, expiry, adverse event, regulated release.
- **Electronics/aerospace/automotive:** counterfeit parts, configuration, functional safety, special processes, supplier change, field action.
- **Chemicals/metals/wood/textiles:** hazardous energy/materials, emissions, grade, treatment, moisture, dye/finish, fire/explosion controls.

Critical exceptions: unapproved substitution, wrong revision, calibration expiry, guard/interlock bypass, contamination, out-of-specification, unexplained yield, counterfeit signal, cyber anomaly, lost genealogy, uncontrolled rework, worker stop, environmental excursion, customer escape, and recall.
