---
triggers: ["Firestore", "google firestore", "gcp firestore", "firestore query", "firestore security rules", "firestore transaction", "cloud datastore"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Firestore

When working with Firestore:

1. Design document structure around your query patterns; nest data in subcollections (`users/{uid}/orders/{orderId}`) for independent pagination and security, but denormalize frequently co-read fields into the parent document to reduce read costs.
2. Create composite indexes proactively with `gcloud firestore indexes composite create --collection-group=orders --field-config=...` for multi-field queries; Firestore will reject queries that lack a matching index, so deploy indexes before code that uses new query patterns.
3. Use transactions for multi-document atomic writes with `db.runTransaction(async (t) => { ... })` in the client SDK; keep transactions under 500 document writes and avoid long-running operations inside the closure to prevent contention and retries.
4. Write security rules that validate data shape and enforce auth: `match /users/{uid} { allow read, write: if request.auth.uid == uid && request.resource.data.keys().hasAll(['name','email']); }` and deploy with `firebase deploy --only firestore:rules`.
5. Use collection group queries (`db.collectionGroup('comments').where(...)`) to query across all subcollections with the same name; ensure a corresponding collection group index exists for the queried fields.
6. Enable TTL policies on documents with `gcloud firestore fields ttls update expireAt --collection-group=sessions --enable-ttl` to automatically delete expired documents and reduce storage costs without cron jobs.
7. For offline persistence in mobile/web apps, enable it with `enablePersistence()` (web) or it is on by default (iOS/Android); handle `fromCache` metadata to show users stale-data indicators and queue writes for sync.
8. Use batched writes (`db.batch()`) for up to 500 operations when you need atomicity without reading first; prefer this over individual writes to reduce round trips and billing on write operations.
9. Implement real-time listeners with `db.collection('chat').onSnapshot((snap) => { snap.docChanges().forEach(...) })` and always process `docChanges()` instead of re-reading the full snapshot to minimize client-side work on incremental updates.
10. Use `FieldValue.arrayUnion()`, `FieldValue.increment()`, and `FieldValue.serverTimestamp()` for atomic field updates instead of read-modify-write patterns; these transform operations avoid transaction overhead for simple mutations.
11. Monitor usage with `gcloud firestore operations list` and the Firestore Usage dashboard; set budget alerts on `firestore.googleapis.com/document/read_count` to catch runaway query costs before they escalate.
12. For Datastore-mode projects, use `gcloud datastore indexes create index.yaml` and structure entity kinds with ancestor paths for strong consistency within entity groups; prefer Firestore native mode for new projects to access real-time features and stronger consistency guarantees.
