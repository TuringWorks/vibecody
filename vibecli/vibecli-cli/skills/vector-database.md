---
triggers: ["vector database", "vector store", "Qdrant", "Pinecone", "pgvector", "Milvus", "Weaviate", "Chroma", "embedding storage"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data
---

# Vector Database

When working with vector databases and embedding storage:

1. **HNSW index tuning** — HNSW (Hierarchical Navigable Small World) is the default index for most vector databases. Tune `m` (connections per node, default 16, higher = better recall but more memory) and `ef_construction` (build-time search width, default 200, higher = better index quality but slower builds). Set `ef` (query-time search width) based on your recall/latency tradeoff — start at 128 and adjust. Higher dimensionality needs higher `m`.

2. **IVF indexes** — IVF (Inverted File Index) partitions vectors into clusters using k-means. Set `nlist` (number of clusters) to roughly `sqrt(N)` for N vectors. At query time, `nprobe` controls how many clusters to search (higher = better recall, slower). IVF is more memory-efficient than HNSW but generally lower recall at the same latency. Use IVF-PQ or IVF-HNSW hybrids for large-scale deployments.

3. **Distance metrics** — Cosine similarity is the standard for normalized text embeddings (most embedding models output normalized vectors). Euclidean (L2) distance works for non-normalized embeddings. Dot product is equivalent to cosine for normalized vectors but faster in some implementations. Choose the metric that matches your embedding model's training objective. Do not mix metrics between indexing and querying.

4. **Collection design** — Create separate collections for semantically different content types (documents, code, images) rather than mixing them. Include a payload/metadata schema with fields you will filter on (tenant_id, source, date, type). Use consistent embedding dimensions across a collection. Plan for growth — estimate vector count and set initial capacity hints.

5. **Metadata filtering** — Combine vector similarity search with metadata filters for precise retrieval. Create payload indexes on frequently filtered fields (string, integer, datetime). Apply filters before or during ANN search depending on the database (pre-filtering is more accurate, post-filtering is faster). Use range filters on dates and numeric fields. Avoid high-cardinality filters that eliminate most candidates.

6. **Batch upsert** — Insert vectors in batches of 100-1000 for optimal throughput. Use upsert (insert or update) with deterministic IDs to handle re-indexing idempotently. Parallelize batch uploads across multiple threads/connections. Monitor indexing lag — some databases defer index updates for performance. Wait for index completion before benchmarking recall.

7. **Hybrid search** — Combine dense vector search with sparse keyword search (BM25 or SPLADE) for best retrieval quality. Qdrant, Weaviate, and Pinecone support hybrid search natively. Use reciprocal rank fusion (RRF) to merge dense and sparse result lists. Weight the combination based on query type — keyword-heavy queries benefit from higher sparse weight.

8. **Replication and sharding** — For high availability, configure replication factor >= 2 (Qdrant: `replication_factor`, Milvus: replica count). For large collections, shard across nodes (Qdrant: `shard_number`, Milvus: shard keys). Replication improves read throughput and fault tolerance. Sharding distributes memory and compute. Monitor shard balance and rebalance if needed.

9. **Backup and restore** — Schedule regular snapshots of collections. Qdrant: use snapshot API (`POST /collections/{name}/snapshots`). Pinecone: use collection backups. pgvector: standard PostgreSQL pg_dump. Store backups in object storage (S3/GCS). Test restore procedures regularly. For critical data, maintain a source-of-truth in a primary database and treat the vector DB as a derived index.

10. **Monitoring** — Track query latency (p50, p95, p99), queries per second, index size, memory usage, and disk usage. Monitor recall quality by periodically running exact search on a sample and comparing to ANN results. Alert on latency spikes, memory pressure, and replication lag. Use the database's built-in metrics endpoint (Prometheus format for Qdrant, Milvus).

11. **Quantization (scalar/product)** — Scalar quantization (SQ) converts float32 vectors to int8, reducing memory 4x with minimal recall loss. Product quantization (PQ) compresses vectors further by splitting into sub-vectors and quantizing each independently. Binary quantization offers 32x compression but higher recall loss. Enable quantization when memory is the bottleneck. Qdrant and Milvus support on-disk quantized indexes with in-memory rescoring.

12. **Multi-tenancy patterns** — For SaaS applications, choose between collection-per-tenant (strongest isolation, higher overhead), partition-by-payload (filter on tenant_id, shared index), or namespace-based (Pinecone namespaces). Payload filtering is the most common approach — create a payload index on tenant_id for fast filtering. Ensure tenant data isolation in queries by always including the tenant filter.
