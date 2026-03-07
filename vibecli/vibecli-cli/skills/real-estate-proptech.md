---
triggers: ["real estate", "PropTech", "property management", "MLS", "RETS", "rental", "lease management", "property valuation", "AVM", "smart building", "tenant portal"]
tools_allowed: ["read_file", "write_file", "bash"]
category: real-estate
---

# Real Estate and PropTech Systems

When working with real estate technology, property management, and PropTech software:

1. Integrate MLS and RETS data feeds reliably — connect to MLS systems via RETS (Real Estate Transaction Standard) or the newer RESO Web API (RESTful, OData-based), handle incremental sync using modification timestamps, normalize listing data across multiple MLS sources into a canonical property schema, manage photo downloads with CDN caching, and respect MLS compliance rules (display requirements, data refresh intervals, attribution).

2. Build automated valuation models (AVM) with transparent methodology — implement comparable sales analysis (select comps by proximity, recency, and similarity; adjust for differences in sqft, bedrooms, condition, lot size), regression-based models (hedonic pricing with property feature coefficients), and repeat-sales indices; combine approaches with ensemble weighting, provide confidence intervals on estimates, and log model inputs for auditability and fair lending compliance.

3. Design property management platforms with multi-entity architecture — model the hierarchy of owners, properties, units, tenants, and leases as distinct entities with relationship mappings; support multiple property types (residential, commercial, mixed-use) with type-specific attributes; implement portfolio-level dashboards aggregating occupancy, revenue, and maintenance metrics across properties.

4. Implement lease management workflows with lifecycle automation — track lease stages (application, screening, approval, execution, active, renewal, termination), automate rent escalation calculations (fixed, CPI-indexed, percentage), generate lease documents from templates with unit-specific merge fields, manage security deposits with jurisdiction-compliant interest calculations, and trigger renewal notices at configurable lead times.

5. Build tenant portal systems with self-service capabilities — provide authenticated tenant access to lease documents, payment history, and maintenance request submission; implement online rent payment with ACH and credit card support (PCI-DSS compliant); enable maintenance request tracking with photo upload and status notifications; and support document sharing (move-in checklists, community rules, insurance certificates).

6. Calculate rent rolls with accounting integration — generate rent rolls showing unit-level detail (tenant, lease dates, base rent, additional charges, concessions, vacancy loss), support accrual and cash-basis accounting views, reconcile with general ledger entries, handle partial month prorations (daily or 30-day method), and export to standard accounting formats (QuickBooks, Yardi, AppFolio).

7. Automate CAM (Common Area Maintenance) reconciliation — calculate tenant CAM shares based on pro-rata square footage or lease-specific formulas, track actual vs. estimated expenses throughout the year, generate annual reconciliation statements with expense category breakdowns, handle caps and exclusions per lease terms, and produce audit-ready documentation of all allocations.

8. Design maintenance work order systems with vendor management — create work orders from tenant requests or preventive maintenance schedules, route to appropriate staff or vendors based on trade/skill and property assignment, track SLA compliance (response time, resolution time), manage vendor contracts and insurance certificates, and build maintenance cost analytics by property, category, and vendor.

9. Integrate smart building IoT systems for operational efficiency — ingest data from building management systems (BACnet, Modbus), smart thermostats, occupancy sensors, water/energy meters, and access control systems; implement rule-based automation (HVAC scheduling, lighting control based on occupancy); generate energy consumption dashboards; and trigger alerts for anomalies (water leaks, HVAC failures, unusual access patterns).

10. Build real estate transaction management workflows — model the deal pipeline (listing, offer, under contract, due diligence, closing) with milestone tracking, manage document checklists (inspection reports, title search, appraisal, loan docs), coordinate multi-party communication (buyer, seller, agents, lender, title company, attorney), track earnest money and escrow, and calculate closing costs with jurisdiction-specific transfer taxes.

11. Implement property search with geospatial queries — use PostGIS or Elasticsearch geo_shape queries for polygon-based search (draw on map), point-radius search, and school district / neighborhood boundary filtering; build faceted search combining location with property attributes (price, beds, baths, sqft, property type); support saved searches with new listing alerts; and optimize for mobile map-based browsing.

12. Design listing syndication to multiple platforms — publish listings to Zillow, Realtor.com, Apartments.com, and other portals via RESO/IDX feeds or platform-specific APIs; maintain listing status synchronization (active, pending, sold) across all channels; handle platform-specific media requirements (photo sizes, virtual tour formats, 3D tour embeds); and track lead attribution by syndication source.

13. Implement investor reporting and analytics — generate property-level and portfolio-level financial reports (income statements, cash flow projections, cap rate analysis, IRR calculations), support waterfall distribution modeling for partnership structures, provide investor portal access with K-1 document distribution, and build scenario analysis tools for acquisition underwriting (sensitivity on rent growth, cap rate, vacancy assumptions).

14. Handle fair housing compliance in software design — ensure property search and tenant screening features comply with Fair Housing Act requirements, avoid discriminatory filtering options, implement adverse action notice generation for denied applications, audit marketing distribution for disparate impact, and maintain records of screening criteria applied consistently across all applicants.
