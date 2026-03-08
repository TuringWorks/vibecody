---
triggers: ["business intelligence", "BI", "data warehouse", "OLAP", "reporting", "analytics platform", "Snowflake", "BigQuery analytics"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data-analytics
---

# Business Intelligence

When building business intelligence and analytics platforms:

1. Design data warehouses using star schema for simplicity and query performance, or snowflake schema when normalization reduces storage and improves data integrity.
2. Choose ELT over ETL for modern cloud warehouses (Snowflake, BigQuery) where compute is elastic; reserve ETL for legacy systems or complex pre-load transformations.
3. Apply dimensional modeling rigorously: define fact tables for measurable events and dimension tables for descriptive context, keeping grain consistent within each fact table.
4. Select the right warehouse platform based on workload: Snowflake for multi-cloud and concurrency, BigQuery for serverless simplicity, Redshift for tight AWS integration.
5. Write analytics SQL using window functions (ROW_NUMBER, LAG, LEAD, running totals), CTEs for readability, and materialized views for frequently accessed aggregations.
6. Deploy self-service BI tools (Looker for governed metrics, Metabase for lightweight exploration, Superset for open-source flexibility) to reduce analyst bottlenecks.
7. Define KPIs through metric trees that connect high-level business outcomes to actionable, measurable indicators owned by specific teams.
8. Establish data governance with a data catalog (DataHub, Atlan), documented ownership, lineage tracking, and access controls to build trust in reporting.
9. Implement slowly changing dimensions using the appropriate SCD type: Type 1 for overwrite, Type 2 for historical tracking with effective dates, Type 3 for limited history.
10. Balance real-time and batch reporting based on business need: use streaming (Kafka, Pub/Sub) for operational dashboards and scheduled batch loads for strategic reporting.
11. Monitor data quality with automated checks on freshness, completeness, uniqueness, and referential integrity; alert on anomalies before they reach dashboards.
12. Democratize data access by providing curated datasets, a semantic layer with business-friendly naming, and training programs so non-technical stakeholders can self-serve.
