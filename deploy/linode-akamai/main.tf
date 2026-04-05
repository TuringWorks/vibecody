terraform {
  required_providers {
    linode = { source = "linode/linode"; version = "~> 2.0" }
  }
}

provider "linode" {
  token = var.linode_token
}

locals {
  tier_config = {
    lite = { type = "g6-standard-2" }   # 2 CPU, 4 GB
    pro  = { type = "g6-standard-4" }   # 4 CPU, 8 GB
    max  = { type = "g6-standard-8" }   # 8 CPU, 16 GB
  }
}

resource "linode_instance" "vibecody" {
  label    = "vibecody"
  type     = local.tier_config[var.tier].type
  region   = var.region
  image    = "linode/ubuntu24.04"
  root_pass = var.root_password

  stackscript_id = linode_stackscript.setup.id

  tags = ["vibecody"]
}

resource "linode_stackscript" "setup" {
  label       = "vibecody-setup"
  description = "Install VibeCody with Docker"
  images      = ["linode/ubuntu24.04"]
  script      = <<-SCRIPT
    #!/bin/bash
    set -e
    apt-get update -qq && apt-get install -y -qq docker.io docker-compose-plugin
    systemctl enable docker && systemctl start docker
    mkdir -p /opt/vibecody && cd /opt/vibecody
    curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/docker-compose.yml -o docker-compose.yml
    docker compose up -d
  SCRIPT
}

resource "linode_firewall" "vibecody" {
  label = "vibecody-fw"
  inbound_policy  = "DROP"
  outbound_policy = "ACCEPT"
  inbound { label = "ssh";  action = "ACCEPT"; protocol = "TCP"; ports = "22"; ipv4 = ["0.0.0.0/0"] }
  inbound { label = "http"; action = "ACCEPT"; protocol = "TCP"; ports = "7878"; ipv4 = ["0.0.0.0/0"] }
  linodes = [linode_instance.vibecody.id]
}
