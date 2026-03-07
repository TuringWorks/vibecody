---
triggers: ["Pub/Sub", "pubsub", "gcp pubsub", "pubsub topic", "pubsub subscription", "google messaging", "pubsub ordering"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Pub/Sub

When working with Pub/Sub:

1. Create topics with `gcloud pubsub topics create TOPIC` and attach schemas using `--schema=SCHEMA_ID --message-encoding=JSON` to enforce message structure at publish time and reject malformed payloads.
2. Use ordering keys by setting `enable_message_ordering=true` on the subscription and publishing with `ordering_key` on each message; note that ordering is per-key, so use a consistent key strategy (e.g., entity ID) to guarantee sequence.
3. Configure dead-letter topics with `--dead-letter-topic=DLT --max-delivery-attempts=5` on subscription creation to capture poison messages instead of blocking the subscription.
4. For exactly-once delivery, create subscriptions with `--enable-exactly-once-delivery` and handle `AcknowledgeResponse` failures by retrying; the client library automatically manages ack IDs that change on redelivery.
5. Prefer pull subscriptions with the `SubscriberClient` streaming pull for high-throughput consumers; set `flow_control.max_messages` and `flow_control.max_bytes` to prevent OOM in consumer processes.
6. For push subscriptions, configure the endpoint with authentication using `--push-auth-service-account=SA_EMAIL` so Pub/Sub signs requests with an OIDC token your endpoint can verify.
7. Use Pub/Sub Lite for cost-sensitive, high-volume workloads by provisioning `gcloud pubsub lite-topics create` with explicit throughput and storage capacity; trade auto-scaling for predictable pricing.
8. Set message retention on the topic with `--message-retention-duration=7d` to enable seek-based replay; use `gcloud pubsub subscriptions seek --time=TIMESTAMP` to replay past messages for recovery.
9. Create BigQuery subscriptions with `--bigquery-table=PROJECT:DATASET.TABLE` to stream messages directly into BigQuery without custom consumers, using `--use-topic-schema` for automatic schema mapping.
10. Batch publish messages using the `PublisherClient` with `batch_settings.max_messages=100` and `max_latency=0.01` to amortize RPC overhead; always check `publish()` futures for individual message failures.
11. Monitor subscription health with the `pubsub.googleapis.com/subscription/oldest_unacked_message_age` metric; alert when it exceeds your SLO to detect stalled consumers before backlog grows.
12. Grant `roles/pubsub.publisher` to producer service accounts and `roles/pubsub.subscriber` to consumer service accounts at the topic/subscription level; never use project-wide `roles/pubsub.admin` for application identities.
