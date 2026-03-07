---
triggers: ["Event Grid", "Event Hubs", "azure event grid", "azure event hubs", "event subscription", "event grid topic", "azure kafka"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Event Grid + Event Hubs

When working with Azure Event Grid and Event Hubs:

1. Use Event Grid for discrete event routing (resource events, custom app events) and Event Hubs for high-throughput streaming (telemetry, logs, clickstream); Event Grid delivers individual events with at-least-once semantics while Event Hubs provides ordered, partitioned streams with configurable retention.
2. Create custom topics with `az eventgrid topic create` and publish events using the SDK: `EventGridPublisherClient(endpoint, AzureKeyCredential(key)).send([EventGridEvent(subject="/orders/123", event_type="Order.Created", data={...})])`; use CloudEvents schema for interoperability with other event systems.
3. Configure event subscriptions with filters to reduce noise: use `--subject-begins-with` and `--subject-ends-with` for path filtering, `--advanced-filter data.amount NumberGreaterThan 100` for payload filtering, and `--enable-advanced-filtering-on-arrays` for array property matching.
4. Validate Event Grid webhook endpoints by handling the `SubscriptionValidation` event: return `{"validationResponse": event.data.validationCode}` in the HTTP response; for Azure Functions, the Event Grid trigger binding handles validation automatically.
5. Implement dead-lettering for failed event deliveries: configure `--deadletter-endpoint` pointing to a Storage blob container; Event Grid retries with exponential backoff (up to 24 hours, 30 attempts by default) before dead-lettering — adjust `--max-delivery-attempts` and `--event-ttl` per subscription.
6. Create Event Hubs namespaces with appropriate throughput units: each TU provides 1 MB/s ingress and 2 MB/s egress; use `--enable-auto-inflate --maximum-throughput-units 20` for variable loads, or Premium/Dedicated tiers for guaranteed capacity and VNet isolation.
7. Partition Event Hubs strategically: choose partition count at creation (cannot change later, 1-32 standard, up to 2048 Dedicated); use `partition_key` on send to co-locate related events, and let the SDK round-robin when ordering is not required — more partitions enable more parallel consumers.
8. Use the `EventProcessorClient` (Python/JS) or `EventProcessorHost` (.NET) for consumer applications: these manage partition ownership via checkpoint store (Blob Storage), handle rebalancing across instances, and resume from last checkpoint on restart — never use raw `EventHubConsumerClient.receive()` in production.
9. Enable Event Hubs Capture for automatic archival: `az eventhubs eventhub update --enable-capture --capture-interval 300 --capture-size-limit 314572800 --destination-name EventHubArchive.AzureBlockBlob` writes Avro files to Blob Storage or Data Lake — zero-code ETL for analytics pipelines.
10. Use the Kafka protocol with Event Hubs (Standard tier and above): configure Kafka clients with `bootstrap.servers=<namespace>.servicebus.windows.net:9093`, SASL mechanism `PLAIN`, and connection string as password — existing Kafka applications work without code changes, simplifying migration.
11. Implement schema validation with Event Grid's schema registry or Event Hubs Schema Registry (`az eventhubs namespace schema-registry`); use Avro serializer in the SDK (`SchemaRegistryAvroSerializer`) to serialize/deserialize events with schema evolution support and compatibility checks.
12. Secure both services with Entra ID RBAC: assign `EventGrid Data Sender` for publishers and `Event Hubs Data Receiver`/`Data Sender` for Event Hubs; use managed identity with `DefaultAzureCredential`, enable private endpoints, and configure diagnostic settings to route operational logs to Log Analytics for monitoring.
