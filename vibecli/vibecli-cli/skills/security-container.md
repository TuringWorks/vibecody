---
triggers: ["container security", "image scanning", "rootless container", "seccomp", "network policy", "container hardening"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Container Security

When securing containers:

1. Scan images for vulnerabilities: `trivy image myapp:latest` — integrate into CI pipeline
2. Use minimal base images: `distroless`, `alpine`, or `scratch` — fewer packages = fewer CVEs
3. Run as non-root: `USER 1001` in Dockerfile — never run containers as root
4. Read-only filesystem: `--read-only` flag + tmpfs for write-needed paths
5. Drop all capabilities: `--cap-drop=ALL`, then add only what's needed with `--cap-add`
6. Use seccomp profiles to restrict system calls — default Docker profile blocks 44+ syscalls
7. Network policies: deny all ingress/egress by default, explicitly allow needed traffic
8. Don't store secrets in images: use Docker secrets, Kubernetes Secrets, or Vault
9. Pin image digests: `FROM node@sha256:abc123` — tags are mutable, digests are not
10. Limit resources: set CPU/memory limits to prevent noisy neighbors and DoS
11. Use multi-stage builds: build tools don't belong in production images
12. Runtime security: use Falco or Sysdig for anomaly detection in running containers
