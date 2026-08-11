---
name: "Industry Taxonomy Router"
description: "Industry Taxonomy Router: Use this skill to prevent category errors before selecting execution skills. Use when the task involves industry taxonomy router."
category: agent
triggers: ["industry taxonomy router"]
tools_allowed: ["read_file", "write_file"]
---

# Industry Taxonomy Router

Use this skill to prevent category errors before selecting execution skills. A national operating system, an industry, an occupation, a software category, and an automation type are separate axes.

## Required reference

Start from the `*-industry-overlay` skills — each states its coverage, priority, and operating-system mappings.

Read the *Reference — Private-Sector Industry Taxonomy and JTBD Coverage Audit* section below when the request involves coverage auditing, category design, an unfamiliar industry, or multi-industry comparison. It contains the detailed 26-category register, gap assessment, common JTBD, automation model, and definition of done.

## Routing workflow

1. **Identify the unit** - company, establishment, business unit, occupation, workflow, product, or software capability.
2. **Classify the establishment** - state the primary good/service produced and select the narrowest supported industry and subsector.
3. **Locate value-chain stages** - R&D, inputs, production, distribution, sales, delivery, service, recovery, and governance as applicable.
4. **Map operating systems** - select every national operating system whose outcomes or infrastructure the work depends on.
5. **Map role families** - identify accountable human owners and advertised job-title families; do not treat an industry label as an occupation.
6. **Map capabilities** - identify systems such as CRM, ERP, EAM, PLM, QMS, WMS, TMS, HRIS, GRC, SIEM, billing, or vertical applications.
7. **Allocate execution** - split tasks among human command, AI personnel, deterministic software, humanoid robots, and non-humanoid autonomous machines.
8. **Load skills** - prefer an industry overlay, then compose operating-system roles and reusable catalogs.
9. **Report gaps** - distinguish absent taxonomy, absent procedural context, absent role skill, absent physical-AI skill, and absent evaluation.

## Classification output

Produce this compact record before doing detailed work:

```yaml
unit:
primary_industry:
subsectors: []
establishment_types: []
business_models: []
value_chain_stages: []
operating_systems: []
human_role_families: []
software_capabilities: []
ai_personnel_candidates: []
physical_ai_candidates: []
human_command_boundaries: []
skills_to_load: []
coverage_gaps: []
```

## Ambiguity rules

- Classify diversified enterprises by establishment, then roll up to the enterprise.
- Classify outsourced work under the provider's industry and also map the client's value-chain stage.
- Treat e-commerce as a channel unless the establishment's primary output is a marketplace or digital intermediary.
- Treat software categories as capabilities. A CRM category does not imply the user is in the software industry.
- Treat AI and robotics as production modalities unless the establishment sells AI or robotics products/services.
- Preserve both formal and informal delivery models.
- Record multiple codes when statistical regimes disagree; do not force false precision.

## Skill composition order

1. Load the industry overlay if one exists.
2. Load the relevant national operating-system orchestrators.
3. Load the smallest set of role or machine skills needed for the task.
4. Load human-command, embodied-AI, fleet, optimization, simulation, or governance catalogs only when their controls are material.
5. Apply the strictest accountability boundary among all loaded skills.

## Completion check

Before claiming an industry is covered, verify its value chain, establishment types, human roles, records, obligations, metrics, exceptions, AI personnel, physical AI, fallback operation, and validation scenarios. A category name alone is inventory, not coverage.

## Reference — Private-Sector Industry Taxonomy and JTBD Coverage Audit

**Status:** Baseline audit, 2026-06-17  
**Scope:** Formal and informal private-sector production, services, infrastructure, and digital markets  
**Library baseline:** 375 skills across 23 national operating systems, 182 sector roles, 31 sector machines/robots, 12 strategic missions, and reusable catalogs

### Executive finding

The library has a strong national-capability backbone, but it is not yet a complete private-sector industry model. Its 23 operating systems describe outcomes a country must produce. Industry classifications describe establishments that produce related goods or services. Occupations describe people and roles. G2 describes software capabilities. These are related but non-interchangeable axes.

The next version should therefore add **industry overlays**, not replace the operating systems. An overlay identifies the subsectors, business models, value-chain stages, establishment types, advertised human roles, AI personnel, physical AI, records, controls, and metrics unique to an industry. It then composes the existing operating-system and catalog skills.

#### Current strengths

- Agriculture, utilities, mining, manufacturing, construction, logistics, communications/software, healthcare, finance, commerce, environment, and public systems all have top-level coverage.
- The library already has strong reusable patterns for AI personnel, autonomous fleets, humanoid robots, embodied AI, capability optimization, human command, simulation, and resilience.
- The universal lifecycle and human-accountability model are suitable for private-sector use.

#### Material gaps

- Establishment-level depth is thin in wholesale distribution, real estate, rental/leasing, professional services, company headquarters, administrative services, facilities services, repair/personal services, forestry, fishing, tourism, restaurants, and many manufacturing subsectors.
- Commercial lifecycle work is scattered: product management, procurement, channel operations, revenue operations, contract management, billing, collections, customer success, field service, and quality management need explicit treatment.
- The library has few industry-specific physical-AI skills outside food, transport, mining, construction, water, healthcare, retail, defense, and public safety.
- It lacks a formal crosswalk among industry, occupation, operating system, software category, and automation modality.
- It lacks explicit coverage registers for establishment types, business models, records, standards, and human licenses.

### Sources and how to use them

This audit uses the following public classification pages as complementary lenses. The pages were supplied by the project owner. Direct automated retrieval was unavailable during this pass, so precise category labels and current page contents should be re-verified before treating the crosswalk as a statistical classification product.

1. [BLS Industries at a Glance, alphabetical index](https://www.bls.gov/iag/tgs/iag_index_alpha.htm) - US establishment and NAICS-oriented industry coverage, labor statistics, and sector/subsector boundaries.
2. [ILO industries and sectors](https://www.ilo.org/topics-and-sectors/industries-and-sectors) - international, labor-centered sector coverage, including maritime, plantations, textiles, hotels/tourism/catering, public services, and extractive industries.
3. [Vertical IQ industry list](https://verticaliq.com/industry-list/) - commercially useful small and midsize business granularity and niche-industry checks.
4. [Simplicable sectors of the economy](https://simplicable.com/en/sectors-economy) - broad conceptual coverage, including primary through quinary activities and public/social domains.
5. [Wikipedia outline of industry](https://en.wikipedia.org/wiki/Outline_of_industry) - broad discovery index for industries and value-chain adjacencies; useful for recall, not as an authoritative standard.
6. [G2 software categories](https://www.g2.com/categories) - enterprise software capability taxonomy. Use it to identify tools and digital workflows, not to classify establishments or occupations.

For implementation, add authoritative crosswalks to ISIC Rev. 4, NAICS 2022, NACE Rev. 2.1, ISCO-08, O*NET/SOC, CPC, and HS/SITC where licensing permits. These provide stable identifiers for industries, occupations, products, and traded goods.

### Taxonomy architecture

Use six orthogonal axes. Never infer one axis solely from another.

| Axis | Answers | Examples |
|---|---|---|
| Economic function | What outcome must the country produce? | food security, mobility, shelter, capital allocation |
| Industry/establishment | What does this establishment primarily produce? | law firm, sawmill, insurer, wholesaler, hotel |
| Value-chain stage | Where does the work occur? | R&D, extraction, conversion, distribution, sales, service, recovery |
| Occupation/role | Who owns or performs the work? | underwriter, machinist, broker, surveyor, dispatcher |
| Capability/tool | What system capability supports it? | CRM, ERP, PLM, EAM, SIEM, payroll, route optimization |
| Automation modality | How may execution be delegated? | AI personnel, deterministic software, robot, vehicle, drone, human |

#### Classification unit

Classify at the **establishment** level when possible. A diversified enterprise may operate factories, warehouses, stores, software teams, a finance subsidiary, and a headquarters. Each establishment can have a different industry code and JTBD profile even when owned by one company.

#### Required crosswalk keys

Every industry overlay should eventually carry:

- Internal industry ID and aliases.
- ISIC, NAICS, and NACE codes where applicable.
- Operating-system dependencies.
- Value-chain stages and product/service outputs.
- SOC/ISCO/O*NET occupation families.
- G2-like software capability categories.
- Applicable product/trade codes for import/export work.
- AI-personnel, humanoid, autonomous-machine, and human-only task tags.

### Comprehensive category register

Coverage states: **Strong** means an orchestrator and useful role set exist; **Partial** means a broad sector exists but material private-sector workflows are absent; **Gap** means no adequate industry treatment exists.

#### 01. Agriculture, livestock, fishing, forestry, and supporting activities

**Subsectors:** field crops; horticulture; controlled-environment agriculture; seed and nursery production; livestock and dairy; poultry and eggs; aquaculture; marine and inland fishing; forestry; logging; hunting/trapping where lawful; farm management; veterinary and breeding support; custom harvesting; soil preparation; irrigation services; post-harvest handling.

**Core JTBD:** plan production; secure land/water/inputs; breed/plant/raise; monitor health; control pests; harvest/catch; grade/store; preserve traceability; sell output; regenerate soil, stocks, and forests.

**Coverage:** Strong for crops and autonomous equipment in OS 05. A cross-biological-production overlay is implemented at `agriculture-livestock-fishing-forestry-industry-overlay`; fishing, aquaculture, seed, cooperative, farm-finance, and support-contractor role depth remains.

#### 02. Mining, quarrying, oil, gas, and extraction support

**Subsectors:** coal; metal ores; critical minerals; stone/sand/clay; oil and gas extraction; drilling and well services; geophysical surveying; mine support; beneficiation; tailings and closure; offshore operations.

**Core JTBD:** discover reserves; secure rights; design extraction; drill/blast/excavate; haul; process; assure grade; manage worker/process safety; maintain equipment; remediate sites; market output.

**Coverage:** Strong general mining and autonomous haulage in OS 08. An extraction overlay is implemented at `mining-quarrying-oil-gas-industry-overlay`; oilfield, quarry, explosives, closure-finance, mineral-trading, and offshore role packs remain.

#### 03. Utilities and network infrastructure

**Subsectors:** electric generation/transmission/distribution; gas utilities; district heating/cooling; water supply; wastewater; irrigation networks; waste collection/treatment; telecommunications networks; data centers; public charging and hydrogen networks.

**Core JTBD:** forecast demand; acquire resources; operate networks; balance flows; meter/bill; inspect/maintain; connect customers; restore service; manage markets; comply; invest for capacity and resilience.

**Coverage:** Strong across OS 06, 07, 12, and 19. A network-utility overlay is implemented at `utilities-network-infrastructure-industry-overlay`; rate cases, trading, vegetation, gas, district-energy, field-workforce, and data-center role packs remain.

#### 04. Construction, engineering construction, and specialty trades

**Subsectors:** residential; commercial/institutional; industrial; roads/bridges; rail/transit; marine works; utilities; demolition/site preparation; concrete/masonry; structural steel; roofing; mechanical/electrical/plumbing; interiors; building envelope; landscaping; modular/prefabricated construction.

**Core JTBD:** originate and estimate; design; permit; procure; mobilize; build; inspect; commission; hand over; manage warranty; maintain safety, schedule, cost, quality, and environmental controls.

**Coverage:** Strong general coverage in OS 10. A construction overlay is implemented at `construction-specialty-trades-industry-overlay`; bids, subcontractor operations, project controls, materials testing, commissioning, claims, modular, and trade-specific packs remain.

#### 05. Manufacturing and industrial production

**Subsectors:** food/beverage/tobacco; textiles/apparel/leather; wood/paper/printing; petroleum/coal; chemicals; pharmaceuticals; rubber/plastics; nonmetallic mineral products; primary/fabricated metals; machinery; computers/electronics/semiconductors; electrical equipment; vehicles; aerospace/rail/shipbuilding; furniture; medical devices; other manufacturing; repair/rebuild.

**Core JTBD:** design products/processes; source inputs; plan capacity; schedule; convert/assemble; control process; test/inspect; package; maintain assets; release product; trace genealogy; improve yield; manage recalls and end of life.

**Coverage:** Strong horizontal factory functions in OS 09, strategic missions, and catalogs. A cross-manufacturing overlay is implemented at `manufacturing-industrial-production-industry-overlay`; subsector process, regulatory, tooling, metrology, maintenance-trade, and specialized-robot packs remain.

#### 06. Wholesale trade, merchant distribution, and trade intermediation

**Subsectors:** durable and nondurable merchant wholesalers; agents and brokers; importers/exporters; industrial distributors; foodservice distribution; pharmaceutical distribution; building-material distribution; electronics distribution; petroleum bulk stations; commodity traders; B2B marketplaces.

**Core JTBD:** select suppliers; negotiate terms; finance inventory; import/export; receive/grade; break bulk; store; price; sell to accounts; extend credit; pick/pack/ship; manage rebates/returns; provide product expertise; control regulated goods.

**Coverage:** Partial across OS 11, 16, and 17. A first deep overlay is implemented at `wholesale-trade-distribution-industry-overlay`; additional regulated-subsector and role depth remains.

#### 07. Retail trade and e-commerce

**Subsectors:** food and beverage stores; pharmacies; fuel/convenience; motor vehicle dealers; building/garden; apparel; electronics; home furnishings; sporting/hobby/book; general merchandise; specialty retail; direct-to-consumer; marketplaces; vending; social commerce.

**Core JTBD:** select assortment; buy; price/promote; allocate inventory; present merchandise; transact; fulfill; prevent loss/fraud; support/retain customers; handle returns; run stores and channels.

**Coverage:** Partial-to-strong in OS 17 with two physical-AI skills. A retail overlay is implemented at `retail-ecommerce-industry-overlay`; pharmacy/dealer, store-labor, loss-prevention, retail-media, and marketplace role packs remain.

#### 08. Transportation, warehousing, postal, courier, and mobility

**Subsectors:** air; rail; ocean; inland water; truck; transit/ground passenger; pipelines; scenic/sightseeing; support services; ports/terminals; freight forwarding; customs brokerage; warehousing; postal; courier/last mile; moving/storage; fleet leasing; mobility platforms.

**Core JTBD:** plan network; sell capacity; accept cargo/passengers; document; schedule/dispatch; move safely; transfer/store; clear borders; deliver; maintain fleets; recover disruptions; settle charges and claims.

**Coverage:** Strong horizontal routing and autonomous vehicle coverage in OS 11. A multimodal overlay is implemented at `transportation-warehousing-postal-mobility-industry-overlay`; aviation, rail control, maritime, ports, forwarding, passenger, and dangerous-goods role packs remain.

#### 09. Information, communications, media, and digital content

**Subsectors:** publishing; motion picture/video; sound recording; broadcasting; telecommunications; computing infrastructure/cloud; data processing/hosting; web search/portals; news; libraries/archives; gaming; creator platforms.

**Core JTBD:** originate/acquire content or data; produce; edit/moderate; package; distribute; monetize; license; protect rights; operate networks/platforms; measure audience; preserve records; maintain trust and safety.

**Coverage:** Strong horizontal digital and media roles in OS 12 and 18. An information/media overlay is implemented at `information-communications-media-content-industry-overlay`; ad-tech, newsroom, production, games/live-ops, telecom field, and provenance role packs remain.

#### 10. Software, IT services, data, cybersecurity, and AI businesses

**Subsectors:** packaged software/SaaS; custom development; systems integration; managed services; cloud platforms; data/analytics; cybersecurity vendors and MSSPs; AI model/platform companies; business-process outsourcing; technical support.

**Core JTBD:** discover needs; manage product; design/build/test; deploy/operate; secure; sell/implement; migrate data; support customers; meter/bill; manage reliability; govern models and third parties.

**Coverage:** Strong engineering agents in OS 12 and frontier missions. A software/AI-business overlay is implemented at `software-it-data-cybersecurity-ai-industry-overlay`; product, UX, solutions, implementation, FinOps, SaaS billing, customer success, and trust/safety role packs remain.

#### 11. Finance, insurance, payments, and capital markets

**Subsectors:** central/commercial/community banking; credit unions; consumer/commercial lending; mortgage; payments; securities/commodities; exchanges; asset/wealth management; venture/private equity; insurance carriers; brokerages/agencies; reinsurance; pensions; fintech; financial-market infrastructure.

**Core JTBD:** acquire and verify customers; price risk; originate; underwrite; transact/custody; invest; service accounts; collect; detect abuse; settle claims; report; manage capital/liquidity; protect consumers and system stability.

**Coverage:** Strong horizontal roles in OS 16. A financial-services overlay is implemented at `finance-insurance-payments-capital-markets-industry-overlay`; treasury, servicing, actuarial, policy administration, claims investigation, fund/investment operations, surveillance, and advisor role packs remain.

#### 12. Real estate, property operations, rental, and leasing

**Subsectors:** residential/commercial brokerage; property management; appraisal; title/escrow; development; real-estate investment; equipment rental; vehicle leasing; consumer-goods rental; intellectual-property and franchise leasing.

**Core JTBD:** source/list assets; value; market; qualify counterparties; contract/close; finance; collect rent; operate/maintain; manage tenants; comply; renew/dispose; optimize portfolio and utilization.

**Coverage:** Partial in OS 10 and 16. A first deep overlay is implemented at `real-estate-rental-leasing-industry-overlay`; title/escrow, development underwriting, and franchise/IP licensing need deeper role packs.

#### 13. Professional, scientific, and technical services

**Subsectors:** legal; accounting/tax/payroll; architecture; engineering; surveying/mapping; design; management consulting; scientific R&D services; advertising/PR; market research; photography/translation; veterinary services; testing laboratories; specialist technical services.

**Core JTBD:** qualify client and matter; define scope; assemble expertise; research/analyze/design; produce defensible deliverables; assure professional quality; communicate advice; manage conflicts/independence; bill/collect; retain knowledge; manage liability.

**Coverage:** Partial across OS 01, 02, 10, 15, 17, and 20. No integrated professional-services operating model. First deep overlay implemented at `professional-scientific-technical-services-industry-overlay`.

#### 14. Management of companies, headquarters, and holding companies

**Subsectors:** corporate headquarters; regional offices; holding companies; conglomerates; shared-services organizations; family offices; portfolio-company operations.

**Core JTBD:** set strategy; allocate capital; govern subsidiaries; manage performance/risk; provide shared services; integrate acquisitions; manage treasury/tax; develop executives; report to owners and regulators.

**Coverage:** Partial. A first deep overlay is implemented at `headquarters-holding-shared-services-industry-overlay`; deeper treasury, tax, entity governance, M&A integration, and shared-service role packs remain.

#### 15. Administrative, employment, facilities, security, and business support services

**Subsectors:** office administration; employment/staffing; contact centers; document preparation; travel arrangement; investigation/security; facilities support; janitorial; landscaping; pest control; packaging/labeling; convention/event services; credit bureaus/collection agencies.

**Core JTBD:** acquire contracts; staff/schedule; execute recurring services; manage access/safety; inspect quality; document proof of service; manage equipment/supplies; invoice; resolve exceptions; comply with labor/privacy/security rules.

**Coverage:** Partial across OS 17, 20, 21, and robot catalogs. A first deep overlay is implemented at `administrative-facilities-security-support-industry-overlay`; staffing, collections, security, and field-service role depth remains.

#### 16. Waste management, remediation, circular economy, and environmental services

**Subsectors:** waste collection; transfer; material recovery; treatment/disposal; hazardous waste; remediation; septic services; recycling brokers; reuse/refurbishment; environmental consulting/testing; carbon and ecosystem services.

**Core JTBD:** characterize waste/site; contract; route/collect; sort/recover; treat/dispose; manifest/trace; protect workers/public; monitor contamination; remediate; verify closure; market recovered materials.

**Coverage:** Partial in OS 19. A waste/remediation overlay is implemented at `waste-remediation-circular-environmental-industry-overlay`; MRF, hazardous-manifest, landfill, remediation-delivery, circular-market, and broker role packs remain.

#### 17. Education, training, credentialing, and knowledge services

**Subsectors:** schools; colleges/universities; vocational/technical; tutoring/test preparation; corporate learning; language schools; driving/flight training; educational support; credentialing/testing; libraries and learning platforms.

**Core JTBD:** diagnose learning need; design curriculum; recruit/enroll; teach/practice; assess; support learners; credential; place graduates; assure quality; conduct research; maintain safe/inclusive institutions.

**Coverage:** Strong learning-agent coverage in OS 14. An education/credentialing overlay is implemented at `education-training-credentialing-industry-overlay`; admissions, registrar, financial aid, institutional research, apprenticeship, placement, and simulation-center role packs remain.

#### 18. Healthcare, life sciences, and social assistance

**Subsectors:** hospitals; physician/dental practices; outpatient; diagnostics/labs; home health; nursing/residential care; behavioral health; pharmacies; health plans; biotech/pharma/medtech; contract research/manufacturing; childcare; disability/community services; emergency/social relief.

**Core JTBD:** prevent; diagnose; treat; monitor; rehabilitate; coordinate; manufacture/distribute therapies; enroll/authorize/pay; protect populations; conduct trials; assure safety/quality; support daily living.

**Coverage:** Strong clinical/public-health support in OS 13 and care support in OS 21. A healthcare/life-sciences overlay is implemented at `healthcare-life-sciences-social-assistance-industry-overlay`; provider operations, revenue cycle, pharmacy, pharmacovigilance, regulatory, CRO/CDMO, home-care, and social-service role packs remain.

#### 19. Arts, entertainment, sports, recreation, and gambling

**Subsectors:** performing arts; spectator sports; promoters/agents; museums/heritage; amusement/theme parks; casinos/gaming; golf/ski/marinas; fitness; outdoor recreation; festivals; esports.

**Core JTBD:** develop talent/content; program events; book venues; sell tickets/rights; stage safely; engage audiences; operate attractions; manage participants; protect integrity; monetize; preserve heritage.

**Coverage:** Partial in OS 18. An arts/sports/recreation overlay is implemented at `arts-entertainment-sports-recreation-gambling-industry-overlay`; venue, performance, integrity, ticketing, gaming, attraction, recreation, and talent role packs remain.

#### 20. Accommodation, food services, tourism, and visitor economy

**Subsectors:** hotels/resorts; short-term accommodation; RV/camps; restaurants; quick service; institutional catering; bars; food trucks; travel agencies; tour operators; destination management; cruise and visitor attractions.

**Core JTBD:** generate demand/reservations; price capacity; receive guests; prepare/serve food; clean/turn space; manage events; maintain safety/hygiene; recover service; coordinate local experiences; manage reputation.

**Coverage:** Partial under OS 17. A first deep overlay is implemented at `accommodation-food-tourism-visitor-economy-industry-overlay`; dedicated reservations, housekeeping, kitchen, event, and tour-operation roles remain.

#### 21. Repair, maintenance, personal, laundry, funeral, and membership services

**Subsectors:** automotive/equipment/electronic repair; commercial machinery maintenance; personal care; laundry/dry cleaning; pet care; funeral services; parking; household services; religious/civic/professional membership organizations.

**Core JTBD:** intake/diagnose; estimate; schedule; repair/service; test; document; return asset; manage parts; maintain dignity/privacy; collect payment; manage memberships and volunteers.

**Coverage:** Partial across OS 18 and 21 plus maintenance roles. A repair/personal/membership overlay is implemented at `repair-personal-membership-services-industry-overlay`; field-service, trade repair, personal-care, laundry, funeral, and association role packs remain.

#### 22. Households, domestic employment, and informal microenterprise

**Subsectors:** domestic workers; household production; family care; street vending; home-based production; day labor; informal transport; waste picking; rotating savings groups; platform/gig work; subsistence production.

**Core JTBD:** coordinate care and household resources; secure income; acquire inputs; produce/sell; manage risk; access services; protect rights; form cooperatives; transition formality by choice without destructive surveillance.

**Coverage:** Strong conceptual treatment in OS 21 and `informal-economy-*`. A household/informal overlay is implemented at `households-informal-microenterprise-industry-overlay`; portable benefits, platform dispute, cooperative back-office, bookkeeping, and locally appropriate tool packs remain.

#### 23. Public administration and state-owned/regulated enterprises

**Subsectors:** executive/legislative; justice; public finance; administration; defense; public safety; social protection; regulators; municipalities; state-owned utilities, transport, banks, and producers.

**Core JTBD:** already represented by the national operating systems. Industry overlays are still needed when a state-owned enterprise competes, contracts, bills, maintains assets, and reports like an establishment.

**Coverage:** Strong public-system coverage. A public/state-enterprise overlay is implemented at `public-administration-state-enterprises-industry-overlay`; enterprise-commercial and regulator-industry interface role packs remain.

#### 24. Nonprofits, foundations, associations, unions, and civil society

**Subsectors:** charities; NGOs; foundations; humanitarian organizations; trade/professional associations; labor unions; advocacy organizations; cooperatives; faith-based service organizations.

**Core JTBD:** define mission; raise funds; manage grants/donors; recruit volunteers/members; deliver programs; advocate; safeguard beneficiaries; measure outcomes; govern; report stewardship.

**Coverage:** Partial across governance, public finance, labor, household, media, and resilience. A nonprofit/civil-society overlay is implemented at `nonprofits-associations-civil-society-industry-overlay`; fundraising, grantmaking, program, volunteer, safeguarding, impact, and membership role packs remain.

#### 25. International trade, border commerce, and global business services

**Subsectors:** import/export merchants; customs brokers; freight forwarders; trade finance; inspection/certification; free zones; bonded warehouses; export promotion; sanctions/export-control services; global payroll/employer-of-record; remittance and foreign-exchange services.

**Core JTBD:** classify goods; screen parties/end use; price landed cost; contract; finance/insure; document origin/value; book transport; declare/clear; inspect; pay duties/taxes; reconcile; manage claims and post-entry audits.

**Coverage:** Partial across OS 03, 11, 16, and 17 plus human-command import/export compliance. An integrated overlay is implemented at `international-trade-global-business-services-industry-overlay`; classification, origin, valuation, licensing, trade-finance, customs-brokerage, and free-zone role packs remain.

#### 26. Frontier and convergent industries

**Subsectors:** semiconductors; advanced batteries; nuclear/fusion; hydrogen; robotics; autonomous systems; space; quantum; synthetic biology; precision medicine; advanced materials; additive manufacturing; climate tech; carbon management; ocean technology.

**Core JTBD:** build scientific advantage; translate research; secure strategic inputs; scale pilot to production; certify safety; create suppliers and talent; protect IP/security; establish standards; finance capacity; compete globally.

**Coverage:** Strong mission-level framing in the 12 strategic missions. A frontier-industry overlay is implemented at `frontier-convergent-industries-industry-overlay`; domain-specific commercialization, certification, field-service, export-control, and production role packs remain.

### Cross-industry private-sector job system

Every overlay must cover these job families even when the industry uses different titles.

1. **Enterprise direction and governance** - strategy, board support, risk appetite, ethics, legal entity, stakeholder management.
2. **Product and portfolio** - market discovery, product/service design, lifecycle, roadmap, pricing, retirement.
3. **Revenue and market access** - marketing, sales, channels, bids, account management, customer success.
4. **Client/customer operations** - intake, onboarding, service delivery, support, complaints, retention.
5. **Supply and procurement** - category strategy, sourcing, contracts, supplier quality, inbound logistics.
6. **Production and operations** - planning, scheduling, execution, supervision, work instructions, proof of completion.
7. **Asset and field service** - commissioning, inspection, preventive/corrective maintenance, parts, warranties.
8. **Quality, safety, and environment** - assurance, testing, release, incident response, corrective action, sustainability.
9. **Finance and capital** - accounting, treasury, tax, planning, credit, billing, collections, investment.
10. **People and organization** - workforce planning, recruiting, learning, scheduling, performance, labor relations.
11. **Legal, compliance, and assurance** - obligations, licenses, records, privacy, audit, investigations, claims.
12. **Technology, data, cyber, and AI** - architecture, engineering, operations, security, analytics, model governance.
13. **Facilities and workplace** - sites, utilities, access, cleaning, space, business continuity.
14. **Knowledge and improvement** - document control, lessons learned, R&D, process improvement, standards.

### Universal private-sector JTBD lifecycle

For each establishment, instantiate the following trigger-response jobs:

1. When deciding where to compete, sense demand and constraints, choose a business model, and allocate accountable capital.
2. When converting an opportunity into an offering, define customer outcomes, requirements, economics, controls, and lifecycle ownership.
3. When capacity is needed, secure people, suppliers, assets, facilities, data, permissions, and financing.
4. When work is accepted, validate identity, authority, scope, terms, risk, conflicts, and ability to perform.
5. When delivery begins, plan, schedule, dispatch, execute, communicate, and preserve evidence.
6. When output is produced, inspect, test, approve, release, hand over, bill, and collect.
7. When expectations are not met, contain harm, recover service, investigate causes, compensate fairly, and improve controls.
8. When conditions change, reforecast, reprice, rebalance capacity, redesign, or exit responsibly.
9. When obligations apply, maintain licenses, controls, records, reporting, auditability, and redress.
10. When automation expands, preserve accountable human command, worker safety, fallback competence, and meaningful appeal.

### Automation allocation model

#### AI personnel: high-fit work

- Research, retrieval, comparison, classification, drafting, coding, translation, forecasting, optimization, scheduling, reconciliation, monitoring, and evidence packaging.
- Routine client intake, document completeness, product-data enrichment, quote preparation, case routing, and status communication within policy.
- Quality prechecks, anomaly detection, obligations mapping, control testing, and structured root-cause support.

#### Deterministic automation: high-fit work

- Calculations, validations, workflow state transitions, access policy, accounting controls, safety interlocks, and high-volume transactions with stable rules.
- Use deterministic systems beneath learned models where exactness, latency, or formal assurance matters.

#### Non-humanoid physical AI: high-fit work

- Vehicles, tractors, harvesters, loaders, cranes, forklifts, AMRs, inspection crawlers, cleaning machines, sorting systems, process equipment, and aerial/surface/underwater drones in bounded operational design domains.

#### Humanoid/mobile manipulators: high-fit work

- Variable human-built environments where doors, shelves, carts, tools, stairs, and mixed object handling make a general-purpose form useful, especially for fetch/carry, kitting, cleaning, inspection, setup, and low-force assistance.

#### Human-only or human-command work

- Fiduciary and professional signoff; consent; high-consequence safety release; coercive action; final hiring/firing; material legal positions; clinical diagnosis/treatment authority; public attestations; conflicts of values; novel exceptions; relationship repair; and accountability for automated systems.

### Required artifact set for every industry overlay

1. **Industry charter** - scope, aliases, codes, outputs, business models, establishment types.
2. **Value-chain map** - upstream inputs, internal transformations, channels, downstream users, recovery/end of life.
3. **JTBD register** - trigger, desired outcome, owner, inputs, outputs, controls, metrics, exceptions.
4. **Role architecture** - advertised titles, seniority, licenses, skills, tools, KPIs, labor-market sources.
5. **AI-personnel roster** - role charter, context pack, tools, permissions, evaluations, escalation.
6. **Physical-AI roster** - environment, ODD, tasks, sensors, safe state, teleoperation, maintenance, evidence.
7. **Record and data model** - systems of record, master data, event logs, retention, lineage, privacy class.
8. **Obligations register** - laws, standards, permits, contracts, professional codes, regulator interfaces.
9. **Control and assurance map** - preventive/detective controls, segregation of duties, release gates, audit tests.
10. **Metrics tree** - outcomes, quality, safety, speed, cost, working capital, trust, workforce impact, resilience.
11. **Scenario and exception library** - normal, edge, fraud, safety, cyber, outage, disaster, and dispute cases.
12. **Keep-warm plan** - human fallback staffing, simulator curriculum, drills, manual mode, recertification.
13. **Implementation roadmap** - data readiness, process maturity, pilots, procurement, change management, scale gates.

### Implementation status

The foundational industry-overlay program is complete: all 26 categories have an overlay, operating-system mappings, accountable human boundaries, curated AI-personnel roles, physical-AI mappings, controls, metrics, failure modes, reference context, and machine-readable index entries. The three original waves below now describe **subsector deepening priorities**, not missing top-level coverage.

Run `python3 skills/examples/07_industry_overlay_audit.py` for the canonical completion check and `06_build_industry_context_pack.py` to exercise any industry by slug.

### Subsector deepening waves

#### Wave 1: highest missing economic leverage

1. Professional, scientific, and technical services.
2. Wholesale distribution and import/export operations.
3. Real estate, rental, and leasing.
4. Administrative, facilities, security, and business support.
5. Accommodation, restaurants, tourism, and visitor services.
6. Headquarters, holding companies, and shared services.

#### Wave 2: deepen broad sectors

1. Manufacturing subsector packs: food, chemicals/pharma, metals/machinery, electronics/semiconductors, vehicles/aerospace, textiles, wood/paper.
2. Transport mode packs: maritime/ports, aviation, rail, trucking, warehousing, courier.
3. Finance packs: banking/lending, insurance, payments, capital markets, wealth/funds.
4. Healthcare/life-sciences packs: provider operations, diagnostics, pharma/biotech, medtech, payer, care services.
5. Agriculture packs: livestock, aquaculture/fishing, forestry/logging, agricultural support.

#### Wave 3: complete the long tail

1. Repair and personal services.
2. Arts, sports, recreation, and gambling.
3. Nonprofits and membership organizations.
4. Circular economy and remediation.
5. Education institution operations.
6. Frontier-industry commercialization packs.

### Skill-production definition of done

An industry is not "covered" merely because its name appears. Mark it complete only when:

- At least 90 percent of its material value-chain stages have explicit JTBD.
- Core establishment types and business models are distinguished.
- Human role families, titles, licenses, tools, and KPIs are grounded.
- At least one deployable AI-personnel pattern exists for each AI-suitable job family.
- Relevant physical work is mapped to humanoid, autonomous machine, conventional automation, or human-only execution.
- Inputs, outputs, records, decision rights, exceptions, and accountability boundaries are specified.
- Safety, security, privacy, labor, professional, and environmental controls are explicit.
- Metrics include outcome quality, unit economics, working capital, safety, trust, resilience, and worker impact.
- Cross-sector dependencies and import/export interfaces are linked.
- Examples and validation scenarios exercise normal operations and high-consequence exceptions.

### Next-depth backlog

Top-level category coverage is complete. Future work should deepen only where a real deployment requires additional procedural specificity:

- Licensed or regulated subsector packs such as pharmacy, aviation, customs brokerage, nuclear, securities, and professional attest services.
- Establishment-specific data schemas, laws, standards, SOPs, and evaluation datasets for a chosen jurisdiction.
- New role skills only when an overlay's existing curated roles cannot execute the work; reuse generic roles instead of cloning them under industry labels.
- Physical-AI safety cases and ODD packs for a named machine, site, route, facility, or production system.
- Forward tests using actual job postings, policies, records, incidents, and operator workflows.

#### Implemented deepening batch

The first regulated and physical-AI subsector batch is complete and indexed in the `*-subsector-pack` skills:

- Customs brokerage and border clearance.
- Commercial aviation and airport operations.
- Pharmacy dispensing and medication fulfillment.
- Nuclear and radiological facility operations.
- Securities trading, clearing, settlement, custody, and surveillance.
- Independent professional attestation engagements.
- Autonomous farm-machine deployment.
- Autonomous freight-corridor deployment.

These eight packs add 20 parent-industry links, 61 operating-system links, explicit licensed-human boundaries, authoritative-record schemas, control gates, metrics, and 80 high-consequence evaluation scenarios. Use a subsector-pack audit script for integrity validation and a subsector context-pack build script for deterministic composition.
