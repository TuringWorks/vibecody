---
triggers: ["MongoDB", "document database", "aggregation pipeline", "mongoose", "mongo index", "sharding mongo"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# MongoDB

When working with MongoDB:

1. Design documents around query patterns — embed related data that's always accessed together
2. Use references (`ObjectId`) for large, independently-accessed subdocuments
3. Create indexes for all query patterns: `db.collection.createIndex({ field: 1 })`
4. Use compound indexes matching your query + sort: `{ status: 1, createdAt: -1 }`
5. Aggregation pipeline: `$match` → `$group` → `$project` → `$sort` — filter early for performance
6. Use `$lookup` for joins — but prefer embedding for frequently-accessed related data
7. Schema validation: use `$jsonSchema` validator on collections for data quality
8. Use `bulkWrite` for batch operations — significantly faster than individual inserts
9. Avoid growing arrays unboundedly — use the bucket pattern for time-series
10. Use `change streams` for real-time event-driven architectures
11. Sharding: choose shard key based on cardinality and write distribution — avoid hotspots
12. Use Mongoose (Node.js) or Motor (Python) for schema validation and middleware hooks
