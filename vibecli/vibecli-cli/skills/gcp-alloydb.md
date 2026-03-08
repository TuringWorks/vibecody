---
triggers: ["AlloyDB", "alloydb", "gcp alloydb", "google alloydb", "alloydb ai", "alloydb omni"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP AlloyDB

When working with AlloyDB:

1. AlloyDB is a PostgreSQL-compatible managed database from Google Cloud — up to 4x faster than standard PostgreSQL for transactional workloads and up to 100x faster for analytical queries.
2. AlloyDB AI: built-in `google_ml_integration` extension for calling Vertex AI models directly from SQL: `SELECT google_ml.predict_row('endpoint_id', json_build_object('instances', json_build_array(...)))`.
3. Columnar engine for analytics: `ALTER TABLE events SET (google_columnar_engine.auto_columnar_enabled = true)` — transparently accelerates analytical queries on OLTP tables without separate OLAP systems.
4. Use `pgvector` for embeddings: `CREATE EXTENSION vector; CREATE INDEX ON docs USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100)` — AlloyDB optimizes vector index performance beyond standard pgvector.
5. Cross-region replication: create read replicas in different regions for disaster recovery and low-latency reads; `gcloud alloydb instances create reader-us-west --region=us-west1 --cluster=my-cluster`.
6. AlloyDB Omni: run the same AlloyDB engine anywhere — on-prem, other clouds, or Kubernetes; `docker run google/alloydbomni` for local development with production-compatible behavior.
7. Automated backups: continuous backup with PITR (point-in-time recovery) up to 14 days; `gcloud alloydb clusters restore --point-in-time '2024-01-15T10:00:00Z'`.
8. Connection: use `gcloud alloydb instances describe` to get IP; connect with Cloud SQL Auth Proxy for IAM-based auth: `cloud-sql-proxy --alloydb alloydb.googleapis.com ...`.
9. Use standard PostgreSQL features: CTEs, window functions, JSONB, full-text search, partitioning, logical replication — AlloyDB is wire-compatible with PostgreSQL 15+.
10. Monitoring: integrated with Cloud Monitoring; key metrics: `database/postgresql/num_backends`, `database/postgresql/transaction/count`, custom query insights in AlloyDB console.
