---
triggers: ["Service Bus", "azure service bus", "service bus topic", "service bus queue", "azure messaging", "service bus session"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Service Bus Messaging

When working with Azure Service Bus:

1. Choose between queues (point-to-point) and topics/subscriptions (pub/sub) based on consumer count; use `ServiceBusClient` with connection string or `DefaultAzureCredential` and always call `close()` or use async context managers to release AMQP connections.
2. Enable sessions (`requires_session=True`) for FIFO ordering and message grouping; set `session_id` on each message and use `ServiceBusSessionReceiver` to lock and process an entire session — ideal for correlated message workflows like order processing.
3. Configure dead-letter queues (DLQ) for poison message handling; after `max_delivery_count` (default 10) attempts, messages auto-move to `$deadletterqueue`; monitor DLQ depth with `az servicebus queue show` and build a DLQ processor that inspects `dead_letter_reason` and `dead_letter_error_description`.
4. Use message deferral (`receiver.defer_message(message)`) when a message arrives out of order; retrieve deferred messages by sequence number with `receiver.receive_deferred_messages(sequence_numbers=[...])` — track sequence numbers in a state store for reliable processing.
5. Schedule messages for future delivery with `sender.schedule_messages(message, schedule_time_utc)` which returns a sequence number; cancel scheduled messages with `sender.cancel_scheduled_messages(sequence_numbers)` — useful for reminder systems and delayed processing.
6. Enable duplicate detection (`requires_duplicate_detection=True`, `duplicate_detection_history_time_window`) at queue/topic creation; set `message_id` on outgoing messages to a deterministic value so the broker silently drops retransmissions within the window.
7. Use `PeekLock` mode (default) over `ReceiveAndDelete` for at-least-once delivery; call `receiver.complete_message(msg)` after successful processing, `receiver.abandon_message(msg)` to retry, or `receiver.dead_letter_message(msg, reason="...")` to explicitly DLQ.
8. Implement batched sending with `ServiceBusSender.create_message_batch()` to pack multiple messages into a single AMQP transfer; check `batch.add_message()` for `MessageSizeExceededError` and send when full — reduces network round-trips and cost.
9. Configure auto-forwarding between queues or from subscriptions to queues (`forward_to` parameter) to build message routing chains without consumer code; combine with subscription filters (`SqlRuleFilter`, `CorrelationRuleFilter`) to route messages by properties.
10. Set message TTL at queue level (`default_message_time_to_live`) and override per-message with `time_to_live` property; expired messages move to DLQ if `dead_lettering_on_message_expiration=True`, otherwise they are silently discarded.
11. Use the AMQP protocol directly for cross-platform clients; Service Bus supports AMQP 1.0 natively — configure `transport_type=TransportType.AmqpOverWebsocket` when behind corporate firewalls that block port 5671, falling back to port 443.
12. Secure with Entra ID roles (`Azure Service Bus Data Sender`, `Azure Service Bus Data Receiver`) instead of connection strings; use managed identity in production, restrict network access with private endpoints, and enable diagnostic logging to capture message flow metrics and failures.
