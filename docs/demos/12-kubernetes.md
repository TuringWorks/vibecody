---
layout: page
title: "Demo 12 — Kubernetes Operations"
permalink: /demos/12-kubernetes/
---

# Demo 12 — Kubernetes Operations

## Overview

VibeCody provides comprehensive Kubernetes management through the CLI and the VibeUI K8s panel. Browse namespaces and pods, view deployment status, scale workloads, stream pod logs, inspect YAML manifests, describe resources, and restart deployments. VibeCody supports 10 dedicated K8s Tauri commands and integrates with `kubectl` for full cluster operations.

---

## Prerequisites

- VibeCody installed (`vibecli --version` returns 0.1+)
- `kubectl` installed and configured with access to a cluster:

```bash
kubectl version --client     # v1.28+
kubectl cluster-info         # Verify cluster connectivity
```

- A Kubernetes cluster (local via minikube/kind/k3d, or remote)
- For VibeUI: `cd vibeui && npm install && npm run tauri dev`

---

## Step-by-Step Walkthrough

### 1. K8s Panel Overview (VibeUI)

**VibeUI:**
1. Open the **K8s** panel (AI Panel > K8s tab)
2. The panel displays:
   - **Namespace selector** — Switch between namespaces
   - **Pods** — Pod list with status, readiness, restarts, age
   - **Deployments** — Deployment list with replica counts and rollout status
   - **Services** — ClusterIP, NodePort, LoadBalancer services with endpoints
   - **Ingresses** — Ingress rules with hosts and paths
   - **ConfigMaps / Secrets** — Configuration data (secrets masked by default)

### 2. List Pods

**CLI:**

```bash
# List pods in the default namespace
vibecli k8s pods

# List pods in a specific namespace
vibecli k8s pods --namespace production

# List pods with wide output (node, IP)
vibecli k8s pods --wide
```

Example output:

```
Pods in namespace: default

NAME                        READY   STATUS    RESTARTS   AGE     NODE
api-server-7d8f9b6c4-x2k9l  1/1    Running   0          2d3h    node-1
api-server-7d8f9b6c4-m4p7n  1/1    Running   0          2d3h    node-2
worker-5c8d7e9f1-q3r8s      1/1    Running   2          5d12h   node-1
redis-0                      1/1    Running   0          7d      node-3
postgres-0                   1/1    Running   0          7d      node-3
```

**REPL:**

```bash
vibecli
/k8s pods
/k8s pods -n production
```

### 3. View Deployments

**CLI:**

```bash
# List deployments
vibecli k8s deploy

# List deployments in all namespaces
vibecli k8s deploy --all-namespaces
```

Example output:

```
Deployments in namespace: default

NAME          READY   UP-TO-DATE   AVAILABLE   AGE
api-server    2/2     2            2           2d3h
worker        1/1     1            1           5d12h
frontend      3/3     3            3           1d8h
```

### 4. Scale Deployments

**CLI:**

```bash
# Scale a deployment
vibecli k8s scale api-server --replicas 4

# Scale in a specific namespace
vibecli k8s scale api-server --replicas 4 --namespace production
```

Example output:

```
Scaling deployment: api-server
  Namespace:     default
  Current:       2 replicas
  Target:        4 replicas
  Status:        Scaling up...

  api-server-7d8f9b6c4-x2k9l  Running (existing)
  api-server-7d8f9b6c4-m4p7n  Running (existing)
  api-server-7d8f9b6c4-a1b2c  Pending -> Running (new)
  api-server-7d8f9b6c4-d3e4f  Pending -> Running (new)

Scale complete: 4/4 replicas ready
```

**REPL:**

```bash
/k8s scale api-server --replicas 4
```

**VibeUI:**
1. In the K8s panel, click a deployment
2. Click the **Scale** button in the deployment detail view
3. Enter the target replica count
4. Click **Apply** and watch pods scale in real time

### 5. Stream Pod Logs

**CLI:**

```bash
# View logs for a pod
vibecli k8s logs api-server-7d8f9b6c4-x2k9l

# Stream logs in real time
vibecli k8s logs api-server-7d8f9b6c4-x2k9l --follow

# View logs for a specific container in a multi-container pod
vibecli k8s logs api-server-7d8f9b6c4-x2k9l --container sidecar

# View logs with timestamps
vibecli k8s logs api-server-7d8f9b6c4-x2k9l --timestamps

# Tail the last 100 lines
vibecli k8s logs api-server-7d8f9b6c4-x2k9l --tail 100

# View logs for all pods in a deployment
vibecli k8s logs --deployment api-server --follow
```

**REPL:**

```bash
/k8s logs api-server-7d8f9b6c4-x2k9l --follow
```

**VibeUI:**
1. Click a pod in the K8s panel
2. The **Logs** tab shows live-streaming output
3. Use the search bar to filter log lines
4. Toggle **Timestamps** and **Wrap** in the toolbar
5. Click **Download** to save the log output

### 6. View Events

**CLI:**

```bash
# View cluster events
vibecli k8s events

# View events for a specific namespace
vibecli k8s events --namespace production

# View events for a specific resource
vibecli k8s events --for pod/api-server-7d8f9b6c4-x2k9l
```

Example output:

```
Events in namespace: default (last 1h)

LAST SEEN   TYPE      REASON              OBJECT                          MESSAGE
2m          Normal    Scheduled           pod/api-server-7d8f9b6c4-a1b2c  Successfully assigned to node-1
2m          Normal    Pulling             pod/api-server-7d8f9b6c4-a1b2c  Pulling image "myapp:latest"
1m          Normal    Pulled              pod/api-server-7d8f9b6c4-a1b2c  Successfully pulled image
1m          Normal    Created             pod/api-server-7d8f9b6c4-a1b2c  Created container api
1m          Normal    Started             pod/api-server-7d8f9b6c4-a1b2c  Started container api
5m          Warning   BackOff             pod/worker-5c8d7e9f1-broken     Back-off restarting failed container
```

### 7. YAML Manifest Viewing

**CLI:**

```bash
# View the YAML manifest for a resource
vibecli k8s yaml deployment/api-server

# View YAML for a pod
vibecli k8s yaml pod/api-server-7d8f9b6c4-x2k9l

# View YAML for a service
vibecli k8s yaml service/api-server
```

**VibeUI:**
1. Click any resource in the K8s panel
2. Click the **YAML** tab to see the full manifest
3. Syntax highlighting and collapsible sections for readability
4. Click **Edit** to modify the YAML and apply changes directly

### 8. Describe Resources

**CLI:**

```bash
# Describe a deployment
vibecli k8s describe deployment/api-server

# Describe a pod
vibecli k8s describe pod/api-server-7d8f9b6c4-x2k9l

# Describe a node
vibecli k8s describe node/node-1
```

Example output (abbreviated):

```
Name:                   api-server
Namespace:              default
Labels:                 app=api-server
Selector:               app=api-server
Replicas:               4 desired | 4 updated | 4 available | 0 unavailable
Strategy:               RollingUpdate (maxSurge: 25%, maxUnavailable: 25%)
Pod Template:
  Containers:
    api:
      Image:        myapp:latest
      Port:         3000/TCP
      Limits:       cpu 500m, memory 512Mi
      Requests:     cpu 250m, memory 256Mi
      Liveness:     http-get /health delay=10s period=30s
      Readiness:    http-get /ready delay=5s period=10s
Conditions:
  Available   True
  Progressing True
Events:
  ScalingReplicaSet  Scaled up to 4
```

### 9. Restart Deployments

**CLI:**

```bash
# Restart a deployment (rolling restart)
vibecli k8s restart deployment/api-server

# Restart in a specific namespace
vibecli k8s restart deployment/api-server --namespace production
```

Example output:

```
Restarting deployment: api-server
  Namespace:  default
  Strategy:   RollingUpdate

  Rolling restart initiated...
  Pod api-server-7d8f9b6c4-x2k9l  Terminating -> Replaced by api-server-8e9f0a7b5-n5p8q
  Pod api-server-7d8f9b6c4-m4p7n  Terminating -> Replaced by api-server-8e9f0a7b5-r2s4t
  Pod api-server-7d8f9b6c4-a1b2c  Terminating -> Replaced by api-server-8e9f0a7b5-u6v8w
  Pod api-server-7d8f9b6c4-d3e4f  Terminating -> Replaced by api-server-8e9f0a7b5-x9y1z

Restart complete: 4/4 new pods running
```

**REPL:**

```bash
/k8s restart deployment/api-server
```

### 10. Services, Ingresses, ConfigMaps, and Secrets

**CLI:**

```bash
# List services
vibecli k8s services
vibecli k8s services --namespace production

# List ingresses
vibecli k8s ingresses

# List configmaps
vibecli k8s configmaps

# List secrets (names only, values masked)
vibecli k8s secrets

# View a specific configmap's data
vibecli k8s describe configmap/app-config

# View a specific secret (base64 decoded)
vibecli k8s describe secret/db-credentials --decode
```

Example services output:

```
Services in namespace: default

NAME          TYPE           CLUSTER-IP     EXTERNAL-IP      PORT(S)          AGE
api-server    LoadBalancer   10.96.45.12    203.0.113.50     80:31234/TCP     2d3h
redis         ClusterIP      10.96.78.90    <none>           6379/TCP         7d
postgres      ClusterIP      10.96.12.34    <none>           5432/TCP         7d
frontend      NodePort       10.96.56.78    <none>           3000:30080/TCP   1d8h
```

---

## Demo Recording

```json
{
  "id": "demo-kubernetes",
  "title": "Kubernetes Operations",
  "description": "Demonstrates the K8s panel, pod management, scaling, log streaming, YAML viewing, resource inspection, and deployment restarts",
  "estimated_duration_s": 200,
  "steps": [
    {
      "action": "Navigate",
      "target": "vibeui://panel/k8s"
    },
    {
      "action": "Narrate",
      "value": "The K8s panel provides a complete view of your Kubernetes cluster. Let's explore pods, scale a deployment, and stream logs."
    },
    {
      "action": "Screenshot",
      "label": "k8s-panel-overview"
    },
    {
      "action": "Assert",
      "target": ".k8s-cluster-status",
      "value": "contains:Connected"
    },
    {
      "action": "Click",
      "target": ".namespace-selector",
      "description": "Open the namespace selector"
    },
    {
      "action": "Click",
      "target": ".namespace-option[data-ns='default']",
      "description": "Select the default namespace"
    },
    {
      "action": "Screenshot",
      "label": "k8s-pods-list"
    },
    {
      "action": "Assert",
      "target": ".pod-row .status-badge",
      "value": "contains:Running"
    },
    {
      "action": "Narrate",
      "value": "We can see all pods in the default namespace with their status, readiness, and restart counts. Let's scale the api-server deployment from 2 to 4 replicas."
    },
    {
      "action": "Click",
      "target": ".k8s-tab[data-tab='deployments']",
      "description": "Switch to Deployments tab"
    },
    {
      "action": "Click",
      "target": ".deployment-row[data-name='api-server']",
      "description": "Click on the api-server deployment"
    },
    {
      "action": "Screenshot",
      "label": "deployment-detail"
    },
    {
      "action": "Click",
      "target": "#scale-btn",
      "description": "Click Scale button"
    },
    {
      "action": "Type",
      "target": "#replica-count-input",
      "value": "4"
    },
    {
      "action": "Click",
      "target": "#apply-scale-btn",
      "description": "Apply the scale"
    },
    {
      "action": "Wait",
      "duration_ms": 3000
    },
    {
      "action": "Screenshot",
      "label": "scaling-in-progress"
    },
    {
      "action": "Narrate",
      "value": "New pods are being scheduled. We can see them transition from Pending to Running. Let's stream logs from one of the pods."
    },
    {
      "action": "WaitForSelector",
      "target": ".deployment-ready-badge[data-ready='4/4']",
      "timeout_ms": 30000
    },
    {
      "action": "Screenshot",
      "label": "scale-complete"
    },
    {
      "action": "Assert",
      "target": ".deployment-ready-badge",
      "value": "contains:4/4"
    },
    {
      "action": "Click",
      "target": ".k8s-tab[data-tab='pods']",
      "description": "Switch back to Pods tab"
    },
    {
      "action": "Click",
      "target": ".pod-row:first-child",
      "description": "Click on the first pod"
    },
    {
      "action": "Click",
      "target": ".pod-detail-tab[data-tab='logs']",
      "description": "Open the Logs tab"
    },
    {
      "action": "Wait",
      "duration_ms": 2000
    },
    {
      "action": "Screenshot",
      "label": "pod-logs-streaming"
    },
    {
      "action": "Narrate",
      "value": "Logs stream in real time with search and timestamp toggle. Now let's view the YAML manifest for this deployment."
    },
    {
      "action": "Click",
      "target": ".k8s-tab[data-tab='deployments']",
      "description": "Switch to Deployments"
    },
    {
      "action": "Click",
      "target": ".deployment-row[data-name='api-server']",
      "description": "Select api-server"
    },
    {
      "action": "Click",
      "target": ".pod-detail-tab[data-tab='yaml']",
      "description": "View YAML manifest"
    },
    {
      "action": "Wait",
      "duration_ms": 1000
    },
    {
      "action": "Screenshot",
      "label": "deployment-yaml"
    },
    {
      "action": "Assert",
      "target": ".yaml-viewer",
      "value": "contains:apiVersion"
    },
    {
      "action": "Narrate",
      "value": "The full YAML manifest is displayed with syntax highlighting. Let's also check the describe output for more details."
    },
    {
      "action": "Click",
      "target": ".pod-detail-tab[data-tab='describe']",
      "description": "View describe output"
    },
    {
      "action": "Wait",
      "duration_ms": 1000
    },
    {
      "action": "Screenshot",
      "label": "deployment-describe"
    },
    {
      "action": "Narrate",
      "value": "The describe view shows conditions, events, pod template details, and resource limits. Let's check events and then restart the deployment."
    },
    {
      "action": "Click",
      "target": ".k8s-tab[data-tab='events']",
      "description": "Switch to Events tab"
    },
    {
      "action": "Screenshot",
      "label": "k8s-events"
    },
    {
      "action": "Click",
      "target": ".k8s-tab[data-tab='deployments']",
      "description": "Back to Deployments"
    },
    {
      "action": "Click",
      "target": ".deployment-row[data-name='api-server'] .restart-btn",
      "description": "Restart the deployment"
    },
    {
      "action": "Click",
      "target": "#confirm-restart-btn",
      "description": "Confirm the rolling restart"
    },
    {
      "action": "Wait",
      "duration_ms": 5000
    },
    {
      "action": "Screenshot",
      "label": "restart-rolling"
    },
    {
      "action": "Narrate",
      "value": "A rolling restart replaces pods one by one, maintaining availability throughout. Let's finish by viewing services and ingresses."
    },
    {
      "action": "Click",
      "target": ".k8s-tab[data-tab='services']",
      "description": "View Services"
    },
    {
      "action": "Screenshot",
      "label": "k8s-services"
    },
    {
      "action": "Click",
      "target": ".k8s-tab[data-tab='ingresses']",
      "description": "View Ingresses"
    },
    {
      "action": "Screenshot",
      "label": "k8s-ingresses"
    },
    {
      "action": "Narrate",
      "value": "VibeCody provides full Kubernetes management from the desktop. All these operations are also available via the CLI with the vibecli k8s command family."
    }
  ],
  "tags": ["kubernetes", "k8s", "pods", "deployments", "scaling", "logs", "devops"]
}
```

---

## What's Next

- [Demo 13 — CI/CD Pipeline](../13-cicd/) — Integrate K8s deployments into CI/CD workflows
- [Demo 11 — Docker & Container Management](../11-docker/) — Build the images you deploy to K8s
- [Demo 15 — Deploy & Database](../15-deploy-database/) — Full deployment workflows with database migrations
