---
triggers: ["SAP", "SAP ERP", "SAP HANA", "SAP FICO", "SAP MM", "SAP SD", "SAP BW", "ABAP"]
tools_allowed: ["read_file", "write_file", "bash"]
category: enterprise
---

# SAP Ecosystem

When working with SAP systems and ABAP development:

1. Understand core SAP modules and their integration points: FI (Financial Accounting) and CO (Controlling) for financials, MM (Materials Management) for procurement, SD (Sales and Distribution) for order-to-cash, PP (Production Planning) for manufacturing, and HR/HCM for human resources — each module shares master data and posts cross-module documents automatically.

2. Follow ABAP development best practices by using ABAP Objects (OO) over procedural code, leveraging Core Data Services (CDS) views for data modeling, applying the clean ABAP style guide, using ABAP Unit for test-driven development, and always checking code with the ABAP Test Cockpit (ATC) before transport.

3. Optimize SAP HANA in-memory performance by designing column-store tables for analytics, using calculation views over nested SQL, pushing down logic to the database layer with AMDP (ABAP Managed Database Procedures), avoiding SELECT * in favor of field lists, and leveraging HANA-specific features like fuzzy search and predictive analysis libraries.

4. Build SAP BW/4HANA reporting solutions using Advanced DSOs as the primary persistence layer, composite providers for virtual data federation, Open ODS views for operational reporting, BW queries with appropriate aggregation levels, and integrate with SAP Analytics Cloud (SAC) for modern dashboards and planning.

5. Design SAP Fiori user experiences following the SAP Fiori design guidelines, choosing the right floorplan (list report, worklist, object page, overview page), using SAP UI5 with OData services, implementing launchpad tiles and target mappings, and ensuring responsive design for desktop, tablet, and mobile users.

6. Implement robust integration using the appropriate technology: RFC (Remote Function Call) for synchronous SAP-to-SAP communication, IDocs for asynchronous EDI and batch data exchange, BAPIs for standardized business object interfaces, OData services for REST-based Fiori and external system access, and SAP Integration Suite (CPI) for cloud-to-cloud and hybrid integration.

7. Manage transport management system (TMS) rigorously by maintaining a three-system landscape (DEV-QAS-PRD), grouping related changes in a single transport request, documenting transport contents and dependencies, never mixing customizing and workbench transports, and using transport of copies for emergency fixes with subsequent retrofit.

8. Configure SAP authorization roles using the principle of least privilege, designing roles with transaction code and authorization object restrictions, using derived roles for organizational-level variants, running SU53 traces for authorization failures, performing periodic user access reviews, and implementing Segregation of Duties (SoD) checks with GRC Access Control.

9. Plan SAP S/4HANA migration by choosing the right approach (greenfield, brownfield, or selective data transition), running the SAP Readiness Check and Simplification Item analysis, addressing custom code remediation with the Custom Code Migration Worklist, planning data volume management before conversion, and establishing a comprehensive testing strategy covering all business processes.

10. Test SAP processes comprehensively using eCATT (extended Computer Aided Test Tool) for automated functional testing, creating reusable test scripts with parameterized variants, testing end-to-end business scenarios across modules, performing regression testing after support packs and enhancements, and integrating with SAP Solution Manager Test Suite for centralized test management.

11. Tune SAP system performance by analyzing SQL traces (ST05), runtime analysis (SE30/SAT), workload analysis (ST03N), monitoring buffer statistics, optimizing ABAP code with the performance trace tools, reviewing table indexes and database statistics, right-sizing application server memory, and using transaction ST04 for database performance monitoring.

12. Leverage SAP Solution Manager for application lifecycle management including project documentation (Solution Documentation), change request management (ChaRM), incident and problem management, monitoring and alerting (Technical Monitoring), custom code lifecycle management, business process monitoring, and root cause analysis for production issues.
