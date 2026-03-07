---
triggers: ["hospitality", "hotel", "PMS", "property management system", "booking engine", "reservation", "OTA", "channel manager", "revenue management", "travel tech", "GDS"]
tools_allowed: ["read_file", "write_file", "bash"]
category: hospitality
---

# Hospitality and Travel Technology Systems

When working with hospitality, hotel management, and travel tech software:

1. Design PMS (Property Management System) architecture around core domains — model reservations (individual, group, block), room inventory (room types, physical rooms, floor plans), guest profiles, housekeeping status (clean, dirty, inspected, out-of-order), and billing folios as distinct bounded contexts; use event-driven communication between domains so that a check-in event triggers housekeeping assignment, minibar setup, and key card activation without tight coupling.

2. Build booking engines with real-time availability and rate calculation — query live room inventory using date-range availability checks that account for existing reservations, out-of-order rooms, and overbooking thresholds; calculate rates by applying rate plans (BAR, corporate, package, promotional) with stay-pattern rules (minimum stay, closed-to-arrival, length-of-stay pricing); and present total stay cost including taxes, fees, and add-ons before confirmation.

3. Integrate channel managers for OTA and GDS distribution — implement two-way sync with OTAs (Booking.com, Expedia, Airbnb) via their APIs and with GDS (Amadeus, Sabre, Travelport) via HTNG or OTA/HTNG XML messaging; maintain a single source of truth for inventory to prevent overbooking; update availability and rates across all channels within seconds of a booking or modification; and track channel-level booking volume, commission costs, and net revenue.

4. Implement revenue management with dynamic pricing algorithms — build demand forecasting models using historical booking pace, day-of-week patterns, local events, and competitor rates; implement yield management rules that adjust rates by room type and channel based on occupancy thresholds and booking window; support overbooking strategies calibrated to historical no-show and cancellation rates; and provide dashboards showing RevPAR, ADR, occupancy, and revenue displacement analysis.

5. Enforce rate parity management across distribution channels — monitor published rates across OTAs, metasearch engines, and direct channels using automated rate shopping; detect and alert on parity violations; implement rate fencing strategies (member-only rates, package bundling, opaque channels) that provide direct booking incentives while maintaining contractual rate parity with OTA partners.

6. Build comprehensive guest profile systems with CRM capabilities — maintain unified guest profiles aggregating data across reservations, loyalty tiers, preferences (room type, pillow, minibar), communication history, and spend patterns; support profile merging for duplicate detection; enable pre-arrival preference confirmation; and power personalized marketing campaigns (pre-arrival upsell, win-back, anniversary) with opt-in consent management.

7. Automate housekeeping workflows with real-time room status — generate daily housekeeping task lists based on departures, stayovers, and arrivals; assign rooms to attendants considering floor zones and workload balance; enable mobile status updates (cleaning started, completed, inspected) that flow to the front desk in real time; track turnover times and quality scores; and handle rush requests and priority overrides for VIP arrivals.

8. Integrate POS (Point of Sale) systems for F&B and spa — connect restaurant, bar, spa, and retail POS systems to the PMS for folio posting (guest charges to room); synchronize menu items and pricing; support split billing and multi-outlet check settlement; track revenue by outlet and period; and enable package inclusion redemption (e.g., breakfast included, spa credit) with automatic POS validation.

9. Design loyalty program engines with tier and points management — implement tier qualification rules (nights, stays, spend per calendar year), points accrual (base + bonus multipliers by tier, rate plan, and channel), redemption catalogs (free nights, upgrades, partner rewards), points expiration policies, and elite benefit fulfillment (late checkout, upgrades, lounge access); support coalition programs and partner earn/burn through API integrations.

10. Implement GDS connectivity for corporate and travel agent bookings — connect to Amadeus, Sabre, and Travelport using HTNG interfaces or certified CRS (Central Reservation System) providers; maintain accurate rate and availability in GDS; support negotiated corporate rate codes, travel agent commission tracking (IATA number validation), and GDS booking delivery to PMS with proper rate plan mapping.

11. Manage availability and inventory with allocation controls — implement inventory pooling (all channels draw from single inventory) with channel-specific allocation overrides; support room type sell controls (open, closed, minimum stay, CTA/CTD) at the rate plan and channel level; handle group blocks with pickup tracking, cutoff dates, and wash factors; and model room type substitution rules for upgrade and downgrade scenarios.

12. Integrate payment gateways with PCI-DSS compliance — tokenize credit cards at the point of capture using PCI-certified payment gateways; implement pre-authorization at booking, incremental auth during stay, and final settlement at checkout; support 3D Secure for online transactions; handle multi-currency with DCC (Dynamic Currency Conversion) options; store only tokens in the PMS; and manage chargebacks with supporting documentation workflows.

13. Build event and banquet management modules — model function spaces (rooms, outdoor areas) with capacity configurations (theater, classroom, banquet, reception), manage event bookings with BEO (Banquet Event Order) generation, track F&B and AV requirements, calculate event pricing (room rental, per-person catering, equipment charges), and coordinate event timelines with housekeeping and engineering for setup and teardown.

14. Implement reporting and business intelligence dashboards — generate standard hospitality reports (manager flash report, daily revenue report, monthly P&L by department, STR competitive set comparison), calculate key metrics (RevPAR, GOPPAR, ADR, occupancy, TRevPAR), support drill-down from portfolio to property to room-type level, and enable data export for ownership reporting and asset management benchmarking.
