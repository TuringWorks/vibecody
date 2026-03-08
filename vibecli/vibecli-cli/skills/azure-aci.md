---
triggers: ["ACI", "azure container instances", "container group", "az container", "aci sidecar", "aci gpu", "azure container instance"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Container Instances

When working with Azure Container Instances:

1. Deploy single containers quickly with `az container create --resource-group rg --name mycontainer --image myimage:tag --cpu 1 --memory 1.5 --ports 80`; use `--restart-policy` set to `Always` for services, `OnFailure` for retries, or `Never` for one-shot batch jobs.
2. Use YAML deployment files for container groups with multiple containers by defining `apiVersion: '2023-05-01'` manifests; apply with `az container create --resource-group rg --file deploy.yaml` for reproducible multi-container deployments.
3. Implement the sidecar pattern by defining multiple containers in a container group sharing `localhost` networking and optional shared volumes; use sidecars for log shipping, TLS termination, or service mesh proxies alongside your main application container.
4. Enable GPU workloads by specifying `--sku GPU` and `--gpu-count` with `--gpu-sku K80/P100/V100` in supported regions; mount model files via Azure Files volumes and set appropriate memory limits for inference workloads.
5. Deploy into a virtual network with `az container create --vnet vnetId --subnet subnetId` for private communication with other Azure resources; the subnet must be delegated to `Microsoft.ContainerInstance/containerGroups` and have a service association link.
6. Assign managed identity using `--assign-identity [system]` or `--assign-identity resourceId` to authenticate with Azure services without credentials; access Key Vault secrets, pull from ACR, and write to Storage using the identity's token endpoint at `169.254.169.254`.
7. Configure confidential containers with `--sku Confidential` and provide a confidential computing enforcement policy; this runs containers in a hardware-backed TEE with encrypted memory, suitable for processing sensitive data without trusting the host.
8. Mount Azure Files shares as volumes using `--azure-file-volume-share-name`, `--azure-file-volume-account-name`, and `--azure-file-volume-account-key`; use this for persistent storage, shared configuration, or output collection across container group restarts.
9. Integrate with Log Analytics by setting `--log-analytics-workspace` and `--log-analytics-workspace-key` at creation; query container logs with KQL in the Azure Portal or via `az monitor log-analytics query` for centralized log management.
10. Set environment variables with `--environment-variables key=value` for non-sensitive config and `--secure-environment-variables key=value` for secrets; secure variables are not visible in the container group properties or API responses after creation.
11. Use liveness and readiness probes in YAML deployments by configuring `livenessProbe` with `httpGet` or `exec` commands, `periodSeconds`, and `failureThreshold`; ACI restarts containers that fail liveness checks automatically.
12. Monitor container group status with `az container show --resource-group rg --name mycontainer --query '{state:instanceView.state, events:instanceView.events}'` and stream logs with `az container logs --follow`; set up Azure Monitor alerts on container restart counts and CPU/memory utilization.
