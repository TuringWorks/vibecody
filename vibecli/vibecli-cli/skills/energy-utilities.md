---
triggers: ["energy", "utilities", "smart grid", "smart meter", "AMI", "SCADA energy", "DERMS", "energy trading", "load forecasting", "demand response", "renewable energy", "microgrid"]
tools_allowed: ["read_file", "write_file", "bash"]
category: energy
---

# Energy & Utilities Systems

When working with energy, utilities, and smart grid systems:

1. Build smart meter data pipelines (AMI - Advanced Metering Infrastructure) that ingest interval usage data (typically 15-minute reads) from head-end systems via MQTT, DLMS/COSEM, or proprietary protocols; normalize readings into a canonical time-series format, detect and flag estimation gaps (missing or implausible reads), and store in a time-series database optimized for high-cardinality meter-level queries at volumes of millions of meters.

2. Integrate SCADA (Supervisory Control and Data Acquisition) systems for grid monitoring by ingesting telemetry from RTUs and IEDs via DNP3 or IEC 61850 protocols; aggregate real-time measurements (voltage, current, power factor, frequency) into a historian database, implement alarm thresholds with deadband filtering to reduce noise, and expose a real-time grid status dashboard with single-line diagram visualization.

3. Implement DERMS (Distributed Energy Resource Management Systems) that register, monitor, and dispatch distributed assets (rooftop solar, battery storage, EVs, demand response loads) through IEEE 2030.5 or OpenADR interfaces; optimize dispatch schedules using constrained optimization (minimize cost or emissions subject to grid constraints), and coordinate with the utility's distribution management system to prevent reverse power flow violations.

4. Design energy trading platforms that handle day-ahead and real-time market participation by submitting bids and offers to ISO/RTO market systems (e.g., via OASIS or MUI interfaces); implement position tracking across forward contracts, spot market, and bilateral trades; calculate mark-to-market P&L in near-real-time; and enforce risk limits (VaR, volume caps) with automatic position alerts.

5. Build load forecasting ML models that predict system load at hourly and sub-hourly granularity using features including historical consumption, weather forecasts (temperature, humidity, cloud cover), calendar variables (day-of-week, holidays), and economic indicators; retrain models on a rolling basis, evaluate accuracy with MAPE and RMSE metrics, and feed forecasts into generation scheduling and procurement decisions.

6. Implement demand response automation that enrolls customer assets in DR programs, dispatches curtailment or load-shift signals via OpenADR 2.0 VTN/VEN architecture, verifies performance against baselines (using approved M&V methodologies such as IPMVP), calculates incentive payments, and provides opt-out mechanisms with advance notification to maintain customer satisfaction.

7. Integrate renewable energy sources by building solar and wind generation forecasting models that combine numerical weather prediction (NWP) data with site-specific historical output; implement ramp-rate controls to smooth intermittent generation, calculate curtailment schedules when generation exceeds grid capacity, and track renewable energy certificates (RECs) for compliance and trading.

8. Develop microgrid control systems that manage islanded and grid-connected modes with seamless transition; implement a local energy management system (EMS) that balances generation, storage, and load within the microgrid boundary using droop control or hierarchical optimization; handle black-start sequencing for resilience, and synchronize reconnection with the main grid via synchrocheck relay coordination.

9. Build outage management systems (OMS) that correlate customer trouble calls and AMI power-status events (last-gasp/power-restoration messages) to identify outage extents on the network model; predict affected customers using connectivity analysis on the GIS-based distribution model, dispatch crew assignments with estimated restoration times, and push outage status updates to customers via SMS, IVR, and web portal.

10. Architect CIS (Customer Information Systems) that manage the full customer lifecycle including service applications, meter-to-premise associations, rate class assignments, billing determinants, bill calculation (demand charges, time-of-use tiers, riders, taxes), payment processing, and collections workflows; support complex rate structures with seasonal and tiered components, and handle net metering credits for solar customers.

11. Automate regulatory reporting for FERC (Federal Energy Regulatory Commission) and NERC (North American Electric Reliability Corporation) compliance by collecting required data elements (generation output, transmission availability, reliability events, CIP cybersecurity evidence) from operational systems, validating completeness against filing requirements, generating reports in mandated formats (e.g., FERC Form 714, NERC GADS), and maintaining audit-ready documentation with version-controlled submissions.

12. Implement Green Button data standards (ESPI - Energy Services Provider Interface) to enable customer access to their usage data; expose a Green Button Connect My Data API (OAuth 2.0 + Atom/XML feeds) that allows authorized third parties to retrieve interval usage on behalf of customers, validate data exports against the ESPI schema, and provide a customer-facing download option for Green Button XML files.

13. Secure operational technology (OT) networks by segmenting IT and OT environments with demilitarized zones, enforcing NERC CIP cybersecurity standards on bulk electric system assets, implementing role-based access to SCADA/EMS with multi-factor authentication, deploying intrusion detection on DNP3/Modbus traffic, and maintaining incident response procedures specific to grid cyber events.

14. Design data architectures that handle the scale of utility operations: partition meter data by meter ID and time range for efficient retrieval, implement data lake patterns for cross-domain analytics (AMI + GIS + OMS + weather), enforce data quality rules at ingestion (range checks, duplicate detection, gap identification), and maintain data lineage tracking for regulatory auditability.
