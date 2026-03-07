---
triggers: ["telecom", "telecommunications", "BSS", "OSS", "billing telecom", "CDR", "call detail record", "provisioning", "network management", "5G", "SIP", "VoIP"]
tools_allowed: ["read_file", "write_file", "bash"]
category: telecom
---

# Telecommunications Core Systems

When working with telecom BSS/OSS and network systems:

1. Architect BSS (Business Support Systems) and OSS (Operational Support Systems) with clear domain boundaries following the TM Forum Open Digital Architecture (ODA): separate billing, CRM, order management, and product catalog on the BSS side from fault, configuration, performance, and security management on the OSS side, using well-defined APIs (TMF Open APIs) for cross-domain integration.

2. Build CDR (Call Detail Record) processing pipelines that ingest raw CDRs from network switches via SFTP or streaming (Kafka), normalize heterogeneous formats (ASN.1, CSV, proprietary) into a canonical schema, apply deduplication and correlation logic, and load into a rated-records store; design for volumes of millions of records per hour with exactly-once processing guarantees.

3. Implement real-time rating and charging engines that evaluate each usage event (voice, data, SMS) against the subscriber's active plan, apply tiered pricing rules, promotional discounts, and roaming surcharges within sub-second latency; maintain an in-memory balance cache with write-through to persistent storage, and support both prepaid (decrement balance) and postpaid (accumulate charges) models.

4. Design subscriber provisioning workflows that activate, modify, suspend, and terminate services across multiple network elements (HLR/HSS, PCRF, DNS, RADIUS/Diameter) in a transactional manner; use an orchestration engine with compensation (saga pattern) to roll back partial activations on failure, and expose a single order-management API to upstream CRM systems.

5. Handle SIP/VoIP call flows by implementing a stateful SIP proxy or B2BUA that manages INVITE/ACK/BYE transactions, supports SDP negotiation for codec selection, handles NAT traversal via SRTP/ICE, and generates CDRs from dialog state transitions; instrument with SIP-level tracing (P-Charging-Vector headers) for end-to-end call correlation.

6. Maintain a network inventory management system that tracks physical assets (towers, cabinets, ports, fiber strands) and logical resources (IP addresses, VLANs, frequency bands) in a unified resource model; enforce referential integrity between logical services and physical infrastructure, and provide topology visualization with GIS integration.

7. Design a service catalog following the TM Forum SID (Shared Information/Data) model, separating product specifications (customer-facing) from resource specifications (network-facing); support composite products (bundles), lifecycle states (draft, active, retired), and version-controlled pricing with effective dates.

8. Build mediation systems that sit between network elements and downstream BSS/OSS, performing protocol translation (SNMP, CORBA, REST, bulk file), data enrichment (e.g., resolving cell IDs to geographic regions), filtering (discard test records), and aggregation (consolidate partial CDRs into complete session records) before forwarding to rating or analytics.

9. Implement number portability by integrating with the national Number Portability Database (NPDB) via standardized interfaces (SOAP/XML or REST); maintain a local routing number cache with TTL-based refresh, intercept call setup to perform dip queries, and handle porting-in/porting-out workflows including donor/recipient carrier coordination and customer notification.

10. Develop 5G network slice management that provisions isolated end-to-end network slices (eMBB, URLLC, mMTC) via NSSF (Network Slice Selection Function) integration; define slice templates with QoS profiles, resource quotas, and SLA parameters; monitor per-slice KPIs and trigger auto-scaling of UPF/SMF instances through the orchestrator when throughput or latency thresholds are breached.

11. Build trouble ticketing systems that integrate with OSS fault management to auto-create tickets from network alarms, correlate related alarms into a single incident (root cause analysis), route tickets based on affected service area and severity, track SLA clocks (response time, resolution time), and provide customer-facing outage status through a portal or API.

12. Implement usage-based billing models that support convergent charging across multiple service types (voice, data, content, IoT); generate itemized invoices with tax calculation per jurisdiction, support partial-period proration for mid-cycle plan changes, handle credit/debit adjustments with full audit trail, and integrate with payment gateways for automatic collection and dunning workflows on failed payments.

13. Ensure regulatory compliance by implementing lawful intercept interfaces (CALEA/ETSI LI) that provision intercept targets without operator knowledge leakage, retain CDR and session data per jurisdiction-specific retention periods, produce regulatory reports (traffic volumes, QoS metrics, coverage maps), and enforce data sovereignty by routing subscriber data through region-appropriate processing nodes.

14. Design for high availability and disaster recovery across all BSS/OSS components: deploy rating engines in active-active clusters with sub-second failover, replicate subscriber databases across geographically separated data centers, implement circuit breakers on all network element integrations, and test recovery procedures quarterly against defined RTO/RPO targets.
