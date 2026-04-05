terraform {
  required_providers {
    digitalocean = { source = "digitalocean/digitalocean"; version = "~> 2.0" }
  }
}

provider "digitalocean" {
  token = var.do_token
}

locals {
  tier_config = {
    lite = { size = "s-2vcpu-4gb",  disk = 40 }
    pro  = { size = "s-4vcpu-8gb",  disk = 80 }
    max  = { size = "s-8vcpu-16gb", disk = 160 }
  }
  tier = local.tier_config[var.tier]
}

resource "digitalocean_droplet" "vibecody" {
  name     = "vibecody"
  image    = "docker-20-04"
  size     = local.tier.size
  region   = var.region
  ssh_keys = var.ssh_key_ids

  user_data = <<-CLOUD_INIT
    #!/bin/bash
    set -e
    apt-get update -qq && apt-get install -y -qq docker-compose-plugin
    mkdir -p /opt/vibecody && cd /opt/vibecody
    cat > docker-compose.yml << 'EOF'
    services:
      vibecli:
        image: ${var.image}
        ports: ["7878:7878"]
        environment:
          - VIBECLI_PROVIDER=ollama
          - OLLAMA_HOST=http://ollama:11434
        depends_on: { ollama: { condition: service_healthy } }
        restart: unless-stopped
        command: ["serve", "--port", "7878", "--host", "0.0.0.0", "--provider", "ollama"]
      ollama:
        image: ollama/ollama:latest
        ports: ["11434:11434"]
        volumes: [ollama-models:/root/.ollama]
        healthcheck:
          test: ["CMD", "curl", "-f", "http://localhost:11434/api/tags"]
          interval: 10s
          timeout: 5s
          retries: 5
          start_period: 30s
        restart: unless-stopped
    volumes:
      ollama-models:
    EOF
    docker compose up -d
  CLOUD_INIT

  tags = ["vibecody"]
}

resource "digitalocean_firewall" "vibecody" {
  name        = "vibecody-fw"
  droplet_ids = [digitalocean_droplet.vibecody.id]
  inbound_rule { protocol = "tcp"; port_range = "22";   source_addresses = ["0.0.0.0/0", "::/0"] }
  inbound_rule { protocol = "tcp"; port_range = "7878"; source_addresses = ["0.0.0.0/0", "::/0"] }
  inbound_rule { protocol = "tcp"; port_range = "443";  source_addresses = ["0.0.0.0/0", "::/0"] }
  outbound_rule { protocol = "tcp"; port_range = "all"; destination_addresses = ["0.0.0.0/0", "::/0"] }
  outbound_rule { protocol = "udp"; port_range = "all"; destination_addresses = ["0.0.0.0/0", "::/0"] }
}
