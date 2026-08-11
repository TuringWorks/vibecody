---
name: "Wholesale Trade and Distribution"
description: "Wholesale Trade and Distribution: This overlay composes OS 03, 08, 11, 16, and 17 into an establishment-level operating model. Use when the task involves wholesale trade and distribution, wholesale trade, distribution."
category: industry
triggers: ["wholesale trade and distribution", "wholesale trade", "distribution"]
tools_allowed: ["read_file", "write_file"]
---

# Wholesale Trade and Distribution

> **Industry ID:** IND-06 · **Accountable human owner:** distribution general manager, trade principal, or licensed compliance owner

This overlay composes OS 03, 08, 11, 16, and 17 into an establishment-level operating model. Read the *Reference — Wholesale Subsectors, Controls, and Exceptions* section below for regulated subsectors, business models, records, and edge cases.

## Mission

Make the right goods available to business customers at the required place, time, condition, documentation, and total cost while controlling product, counterparty, inventory, credit, trade, safety, and channel risk.

## Establishment archetypes

- Stocking merchant wholesaler or industrial distributor.
- Importer/exporter, export management company, or trading house.
- Commission agent, broker, manufacturer's representative, or buying group.
- Foodservice, pharmaceutical, petroleum, chemical, electronics, or building-material distributor.
- Commodity merchant, bulk terminal, or bonded/free-zone operator.
- Dropshipper, B2B marketplace, or digitally enabled distributor.

## Core Jobs To Be Done

1. When choosing a market, define customer segments, assortment, service levels, channels, territories, and unit economics.
2. When onboarding suppliers or products, verify identity, rights, specifications, provenance, quality, compliance, capacity, and commercial terms.
3. When importing or exporting, classify goods, determine origin/value, screen parties/end use, obtain licenses, finance/insure, document, declare, and preserve evidence.
4. When planning inventory, forecast demand, set stocking policy, place orders, finance working capital, and manage shelf life or obsolescence.
5. When goods arrive, schedule, receive, inspect, quarantine exceptions, record lot/serial genealogy, and put away safely.
6. When customers buy, validate account, contract, tax, price, availability, credit, allocation, export destination, and delivery promise.
7. When fulfilling, reserve, pick, pack, stage, document, transport, track, deliver, and capture proof without breaking chain of custody.
8. When supply is constrained, allocate fairly under approved policy, communicate backorders, source alternatives, and escalate strategic customers or public-interest goods.
9. When transactions settle, reconcile receipts, rebates, commissions, freight, duties, invoices, deductions, returns, claims, and collections.
10. When products fail or become unsafe, stop shipment, trace affected units, notify accountable parties, recall/return, investigate, and prevent recurrence.

## Operating lifecycle

| Stage | Required outputs | Human owner |
|---|---|---|
| Market and assortment | segment, category strategy, service/economic model | commercial leader |
| Supplier/product approval | due diligence, specification, terms, product master | category and quality owners |
| Trade and inbound | classification, licenses, documents, bookings, landed cost | trade compliance/logistics owner |
| Plan and stock | forecast, order, safety stock, working-capital plan | inventory owner |
| Receive and control | receipt, inspection, genealogy, quarantine, putaway | warehouse/quality owner |
| Sell and promise | quote/order, credit, tax, allocation, delivery promise | sales/credit owner |
| Fulfill and deliver | pick/pack/ship, proof, exception record | distribution owner |
| Settle and support | invoice, rebate, collection, return, claim, advice | finance/account owner |

## AI personnel allocation

- Perform product-data enrichment, demand sensing, replenishment proposals, order validation, landed-cost calculation, document completeness, screening support, slotting, routing, allocation simulation, invoice matching, deduction classification, and customer status updates within policy.
- Recommend supplier selection, assortment, price, credit, constrained allocation, substitutions, expedite decisions, and claim disposition.
- Escalate sanctions/export-control concerns, controlled goods, dangerous goods, counterfeit or provenance issues, quality release, material credit exposure, unusual routing/payment, product safety, bribery indicators, or policy exceptions.

## Physical AI allocation

- Warehouse AMRs, autonomous forklifts, conveyors, sorters, palletizers, inventory drones, dimensioners, and robotic picking for bounded facilities.
- Autonomous yard tractors, freight trucks, port equipment, and delivery vehicles within approved ODDs and teleoperation coverage.
- Inspection drones and sensor systems for tanks, racks, roofs, yards, bulk inventory, and inaccessible infrastructure.
- Keep hazardous-product handling, unverified loads, damaged containers, confined spaces, lockout/tagout, and novel exceptions human-led or under direct specialist control.

## Human accountability boundary

Humans must own supplier and customer acceptance; binding commercial terms; controlled-product authorization; customs declarations and material classifications where law assigns responsibility; sanctions/export-license decisions; credit limits and write-offs; constrained allocation policy; quality release; dangerous-goods acceptance; recall; fraud/bribery response; worker safety; material claims; and regulator, insurer, supplier, or customer notification.

## Systems of record

ERP/order management; CRM/CPQ; product information and master data; supplier management; WMS/yard management; TMS/freight audit; global trade management; customs broker portal; quality/lot/serial traceability; credit/collections; rebate/commission management; EDI/API/B2B marketplace; document/records management; fleet/maintenance/telemetry.

## Controls

- Segregate vendor setup, purchasing, receipt, payment, customer credit, shipping, refunds, and write-offs.
- Bind every transaction to approved parties, products, terms, locations, tax/trade treatment, and evidence.
- Prevent shipment when license, screening, quality, temperature, lot, serial, dangerous-goods, or credit gates fail.
- Reconcile physical, perpetual, customs/bonded, consignment, and financial inventory.
- Preserve country-of-origin, classification, valuation, end-use, chain-of-custody, and recall evidence.
- Independently verify AI-generated classifications, prices, documents, substitutions, and allocations by risk tier.

## Metrics

Service: fill rate, OTIF, backorder age, perfect order, complaint/return rate. Economics: gross margin, inventory turns, GMROI, landed-cost variance, rebate realization, DSO, bad debt. Operations: dock-to-stock, pick accuracy, cost per order/line, damage, shrinkage, capacity. Risk: screening/licensing exceptions, customs amendments, traceability time, recalls, safety events, counterfeit/provenance incidents. Automation: correction rate, exception precision, unauthorized-action rate, safe-stop and teleoperation performance.

## Failure modes and keep-warm

- False product matches, stale master data, incorrect origin/classification, hidden channel conflict, unfair allocation, phantom inventory, counterfeit substitution, temperature excursions, autonomous-equipment congestion, and optimization that sacrifices safety or customer commitments.
- Preserve manual order entry, inventory counts, trade-document preparation, allocation judgment, warehouse recovery, and customer communication through drills and sampled human execution.

## Operating procedure

1. Classify subsector, establishment model, products, jurisdictions, channels, and service promises.
2. Name commercial, trade, quality, credit, warehouse, and safety owners.
3. Map supplier-to-customer value flow, records, custody, money, and decision gates.
4. Define product/customer/supplier master-data authority and exception policy.
5. Allocate bounded tasks to AI, deterministic systems, warehouse automation, vehicles, and humans.
6. Test normal, constrained-supply, controlled-goods, counterfeit, recall, cyber, outage, and equipment-failure scenarios.
7. Deploy by risk tier with evidence logs, release gates, incident response, and manual fallback.

## Reference — Wholesale Subsectors, Controls, and Exceptions

### Subsector modifiers

- **Foodservice/perishables:** cold chain, shelf life, food defense, allergen/lot traceability, substitutions, recall speed.
- **Pharmaceutical/medical:** authorized trading partners, serialization, pedigree, controlled substances, storage conditions, suspect product.
- **Chemicals/petroleum:** SDS, dangerous goods, tank compatibility, quantity measurement, environmental release, emergency response.
- **Electronics:** counterfeit components, export controls, allocation, lifecycle/obsolescence, serial traceability, warranty.
- **Building/industrial:** technical selection, project delivery windows, jobsite safety, cut-to-length/configuration, returns/restocking.
- **Commodities:** grade, assay, weight, title, hedging, demurrage, storage loss, sanctions, market-conduct controls.
- **Agents/brokers:** disclosed authority, commission, conflicts, principal instructions, no unauthorized custody or representations.

### Commercial models

Stock-and-resell; consignment; vendor-managed inventory; dropship; agency/commission; exclusive territory; buying group; marketplace; private label; bulk terminal; bonded/free-zone; forward contract and commodity trading.

### Critical exceptions

Unknown beneficial owner; denied party or controlled end use; classification/origin disagreement; missing license; related-party valuation; damaged seal; quantity/grade discrepancy; unexpected temperature; counterfeit signal; expired/recalled lot; oversold stock; allocation dispute; diversion request; unusual payment/routing; negative margin; duplicate rebate; customer insolvency; cyber compromise of order or bank details.

### Curated role composition

Wholesale assortment/replenishment; distribution/allocation; customs documentation; demand forecast; warehouse slotting; routing; disruption response; KYC/AML; credit memo; reconciliation; vendor risk; product quality and traceability; human import/export compliance owner.

### Physical evidence

Capture receipt identity, seal/container, quantity/weight/dimensions, condition, temperature, lot/serial, location moves, picks, pack, load, departure, custody transfers, delivery, return, quarantine, and destruction with calibrated sensors and tamper-evident logs.
