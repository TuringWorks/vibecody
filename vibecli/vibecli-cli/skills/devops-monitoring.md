---
triggers: ["Prometheus", "Grafana", "OpenTelemetry", "SLO", "SLI", "alerting", "observability", "metrics monitoring"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Monitoring & Observability

When implementing monitoring and observability:

1. Three pillars: metrics (counters, gauges, histograms), logs (structured JSON), traces (distributed)
2. Use OpenTelemetry SDK for vendor-neutral instrumentation — export to any backend
3. Define SLIs (Service Level Indicators): latency p99, error rate, throughput
4. Set SLOs (Service Level Objectives): "99.9% of requests complete in < 200ms"
5. Alert on SLO burn rate, not individual metrics — reduces alert fatigue
6. Use RED method for services: Rate, Errors, Duration
7. Use USE method for resources: Utilization, Saturation, Errors (CPU, memory, disk, network)
8. Prometheus: use histograms for latency, counters for totals, gauges for current values
9. Grafana dashboards: one per service — include golden signals (traffic, errors, latency, saturation)
10. Structured logging: JSON format with `timestamp`, `level`, `message`, `trace_id`, `service`
11. Distributed tracing: propagate context (trace ID) across service boundaries via headers
12. Page on symptoms (service down, error spike), not causes (high CPU, low disk)
