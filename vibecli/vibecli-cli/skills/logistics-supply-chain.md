---
triggers: ["supply chain", "logistics", "warehouse", "WMS", "TMS", "inventory", "fulfillment", "last mile", "route optimization", "freight", "shipping", "procurement"]
tools_allowed: ["read_file", "write_file", "bash"]
category: logistics
---

# Logistics and Supply Chain Systems

When working with logistics, warehouse, and supply chain software:

1. Design WMS (Warehouse Management System) around zone-based architecture — model warehouse layouts as hierarchical zones (dock, receiving, storage, pick, pack, ship) with bin-level granularity, support multiple storage strategies (fixed, random, class-based), and track inventory movements as immutable transactions for full traceability.

2. Implement inventory management with safety stock and reorder points — calculate safety stock using demand variability and lead time uncertainty (SS = Z * sqrt(LT * demand_variance + demand_avg^2 * LT_variance)), set reorder points as (average_daily_demand * lead_time + safety_stock), and support ABC/XYZ classification to differentiate replenishment strategies by SKU importance.

3. Solve route optimization using VRP (Vehicle Routing Problem) algorithms — implement Clarke-Wright savings algorithm for initial solutions, improve with metaheuristics (tabu search, simulated annealing, or genetic algorithms), respect constraints (vehicle capacity, time windows, driver hours-of-service), and integrate real-time traffic data for dynamic re-routing.

4. Integrate TMS (Transportation Management System) via EDI and API — support EDI 204 (load tender), 214 (shipment status), 210 (freight invoice), and 990 (response to load tender) for carrier communication; implement REST APIs for modern carrier integrations; and maintain a carrier scorecard tracking on-time delivery, damage rates, and cost per mile.

5. Build order fulfillment workflows with wave and waveless picking — support wave planning (grouping orders by carrier cutoff, zone, or priority) and waveless continuous flow picking; implement pick path optimization (serpentine or shortest-path through warehouse zones); and handle exceptions (short picks, substitutions, backorders) with configurable business rules.

6. Optimize last-mile delivery with dynamic dispatch — cluster deliveries by geographic proximity using k-means or DBSCAN, assign to drivers considering capacity and shift constraints, provide real-time ETA updates using GPS telemetry, support proof-of-delivery (photo, signature capture), and enable customer self-service rescheduling.

7. Build demand forecasting models with ensemble methods — combine time-series methods (ARIMA, exponential smoothing) with ML approaches (gradient boosting, LSTM) for demand prediction; incorporate external signals (seasonality, promotions, weather, economic indicators); and measure forecast accuracy using MAPE, bias, and weighted absolute percentage error.

8. Automate procurement with approval workflows and supplier scoring — implement purchase requisition to PO conversion with configurable approval chains (amount thresholds, category-based routing), maintain supplier scorecards (quality, delivery, cost, responsiveness), support punch-out catalogs (cXML), and automate three-way matching (PO, receipt, invoice).

9. Design barcode and RFID tracking systems for real-time visibility — support GS1-128 and GS1 DataMatrix barcodes for item-level tracking, implement RFID read-point architecture for pallet and case-level tracking at dock doors and zone transitions, handle read deduplication and filtering, and publish location events to an event stream for downstream consumers.

10. Implement carrier rate shopping with multi-modal support — build a rate engine that queries multiple carriers (parcel, LTL, FTL, air, ocean) in parallel, normalizes quotes to a comparable format (accounting for accessorials, fuel surcharges, dimensional weight), ranks by cost/transit-time tradeoff, and supports business rules for carrier allocation (e.g., minimum volume commitments).

11. Design cross-docking logic to minimize warehouse dwell time — identify cross-dock-eligible shipments based on inbound/outbound schedule alignment and order urgency, route receiving directly to outbound staging bypassing put-away, synchronize inbound and outbound dock door assignments, and track cross-dock efficiency as a percentage of total throughput.

12. Build supply chain visibility platforms with event-driven architecture — publish milestone events (PO confirmed, shipped, in-transit, customs cleared, delivered) to a central event bus, aggregate across suppliers and carriers into a unified tracking timeline, calculate ETA predictions using historical transit data, and trigger exception alerts for delays exceeding configurable thresholds.

13. Handle returns and reverse logistics workflows — design RMA (Return Merchandise Authorization) processes with disposition rules (restock, refurbish, recycle, dispose), track return shipments with carrier integration, automate refund/replacement triggers based on inspection outcomes, and feed return data back into quality analytics.

14. Implement lot tracking and expiration management — assign lot numbers at receiving, enforce FIFO/FEFO (First Expired First Out) picking rules, generate expiration alerts at configurable lead times, support lot recall workflows that trace affected inventory across all warehouse locations and shipped orders, and maintain chain-of-custody records for regulated goods.
