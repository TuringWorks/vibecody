---
triggers: ["OpenFaaS", "openfaas", "faas-cli", "openfaas template", "openfaas function", "faasd", "openfaas kubernetes"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# OpenFaaS Functions-as-a-Service

When working with OpenFaaS:

1. Install OpenFaaS on Kubernetes using `arkade install openfaas` or the official Helm chart; retrieve the gateway credentials with `echo $(kubectl get secret -n openfaas basic-auth -o jsonpath="{.data.basic-auth-password}" | base64 --decode)` and log in with `faas-cli login`.
2. Create new functions with `faas-cli new --lang python3-http myfunction` which scaffolds a handler directory with `handler.py` and `requirements.txt`; choose language templates matching your runtime — `python3-http`, `node18`, `go`, `csharp`, `dockerfile` for custom images.
3. Use the `of-watchdog` (HTTP mode) template variants for production workloads; they keep the process warm between invocations, support request/response streaming, and offer 10-100x lower latency compared to the classic `fwatchdog` fork-per-request model.
4. Build and deploy with `faas-cli up -f stack.yml` which runs build, push, and deploy in sequence; configure `stack.yml` with function-level environment variables, secrets, labels, resource limits, and scaling parameters.
5. Configure auto-scaling by setting labels `com.openfaas.scale.min`, `com.openfaas.scale.max`, and `com.openfaas.scale.target` (requests per second target); the built-in alerting with Prometheus triggers scale-up, and the idle scaler handles scale-to-zero.
6. Handle async invocations by calling the gateway with `X-Callback-Url` header or using the `/async-function/` endpoint; OpenFaaS queues the request via NATS and delivers the result to the callback URL, decoupling request submission from execution.
7. Mount Kubernetes secrets into functions by creating secrets with `faas-cli secret create mysecret --from-file=key.pem` and referencing them in `stack.yml` under `secrets:`; they appear as files under `/var/openfaas/secrets/` inside the function container.
8. Use `faasd` for lightweight single-node deployments on VMs or edge devices; install with the faasd installation script, which runs functions as containerd containers without Kubernetes overhead while maintaining full `faas-cli` compatibility.
9. Create custom function templates with `faas-cli template pull <repo>` or build your own in `template/<name>/` with a `Dockerfile` and `template.yml`; custom templates enable standardized base images with security patches, monitoring agents, or company-specific middleware.
10. Monitor functions using the built-in Prometheus metrics at the gateway; track `gateway_function_invocation_total`, `gateway_functions_seconds` (latency histogram), and `gateway_service_count` (replicas) — import the OpenFaaS Grafana dashboard for visualization.
11. Implement function chaining by having one function invoke another through the gateway URL (`http://gateway.openfaas:8080/function/<name>`); for complex workflows, use OpenFaaS Pro's function builder or external orchestrators like Argo Workflows.
12. Set resource limits in `stack.yml` with `limits: { memory: 128Mi, cpu: 100m }` and `requests: { memory: 64Mi, cpu: 50m }` to prevent noisy neighbors; configure `exec_timeout` and `read_timeout` on the watchdog to enforce function execution time bounds.
