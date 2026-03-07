---
triggers: ["DigitalOcean", "digitalocean", "droplet", "app platform", "DOKS", "digitalocean spaces", "doctl", "digitalocean database"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["doctl"]
category: cloud-do
---

# DigitalOcean

When working with DigitalOcean:

1. Authenticate the doctl CLI with `doctl auth init --access-token $DO_API_TOKEN` and verify with `doctl account get`; for CI/CD, set the `DIGITALOCEAN_ACCESS_TOKEN` environment variable and use `doctl auth init --access-token $DIGITALOCEAN_ACCESS_TOKEN` non-interactively.
2. Create Droplets with `doctl compute droplet create myserver --region nyc3 --size s-2vcpu-4gb --image ubuntu-24-04-x64 --ssh-keys $KEY_FINGERPRINT --tag-name web --user-data-file cloud-init.yaml` and use cloud-init scripts to automate provisioning on first boot.
3. Deploy applications on App Platform with `doctl apps create --spec app.yaml` where the spec defines services, workers, jobs, databases, and environment variables; update with `doctl apps update $APP_ID --spec app.yaml` for GitOps-style deployments.
4. Provision managed databases with `doctl databases create mydb --engine pg --version 16 --region nyc3 --size db-s-1vcpu-1gb --num-nodes 1` and configure connection pools: `doctl databases pool create $DB_ID mypool --db defaultdb --mode transaction --size 20 --user doadmin`.
5. Set up DOKS Kubernetes clusters with `doctl kubernetes cluster create prod --region nyc3 --version 1.29.1-do.0 --node-pool "name=workers;size=s-4vcpu-8gb;count=3;auto-scale=true;min-nodes=2;max-nodes=5"` and save kubeconfig with `doctl kubernetes cluster kubeconfig save prod`.
6. Create and manage Spaces (S3-compatible object storage) with `doctl compute cdn create --origin myspace.nyc3.digitaloceanspaces.com` for CDN-backed delivery, and use the AWS SDK with endpoint `https://nyc3.digitaloceanspaces.com` and Spaces access keys for programmatic uploads.
7. Deploy serverless Functions with `doctl serverless connect` and `doctl serverless deploy` from a project directory containing `project.yml`; define functions with runtime, memory, and timeout: `functions: - name: hello, runtime: nodejs:18, limits: { memory: 256, timeout: 5000 }`.
8. Configure load balancers with `doctl compute load-balancer create --name web-lb --region nyc3 --forwarding-rules "entry_protocol:https,entry_port:443,target_protocol:http,target_port:8080,certificate_id:$CERT_ID" --droplet-ids $IDS` for TLS termination at the edge.
9. Use the DigitalOcean Terraform provider: `provider "digitalocean" { token = var.do_token }` with resources like `digitalocean_droplet`, `digitalocean_kubernetes_cluster`, `digitalocean_database_cluster`, and `digitalocean_spaces_bucket` for reproducible infrastructure.
10. Set up monitoring and alerts with `doctl monitoring alert create --type v1/insights/droplet/cpu --compare GreaterThan --value 80 --window 5m --entities $DROPLET_ID --emails ops@example.com` to get notified before resource exhaustion impacts users.
11. Optimize costs by using Reserved Droplets for long-running workloads (save up to 30%), right-sizing with `doctl compute droplet actions resize $ID --size s-1vcpu-1gb --resize-disk=false` (non-permanent resize), and destroying unused resources with `doctl compute droplet delete-by-tag temp`.
12. Secure infrastructure by configuring VPC networks (`doctl vpcs create --name private --region nyc3 --ip-range 10.10.10.0/24`), enabling Cloud Firewalls (`doctl compute firewall create --name web-fw --inbound-rules "protocol:tcp,ports:443,address:0.0.0.0/0"` --droplet-ids $IDS), and restricting database trusted sources to VPC-only access.
