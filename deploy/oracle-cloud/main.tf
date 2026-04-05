terraform {
  required_providers {
    oci = { source = "oracle/oci"; version = "~> 5.0" }
  }
}

provider "oci" {
  region = var.region
}

locals {
  tier_config = {
    lite = { ocpus = 2, memory_gb = 4 }
    pro  = { ocpus = 4, memory_gb = 8 }
    max  = { ocpus = 4, memory_gb = 24 }  # Always-free max: 4 OCPU, 24 GB
  }
  tier = local.tier_config[var.tier]
}

# Always-free ARM container instance
resource "oci_container_instances_container_instance" "vibecody" {
  compartment_id       = var.compartment_id
  availability_domain  = var.availability_domain
  display_name         = "vibecody"
  shape                = "CI.Standard.A1.Flex"  # ARM (always-free eligible)

  shape_config {
    ocpus         = local.tier.ocpus
    memory_in_gbs = local.tier.memory_gb
  }

  containers {
    display_name = "vibecli"
    image_url    = var.image
    environment_variables = {
      VIBECLI_PROVIDER = "ollama"
      OLLAMA_HOST      = "http://localhost:11434"
      RUST_LOG         = "info"
    }
    health_checks {
      health_check_type = "HTTP"
      port              = 7878
      path              = "/health"
      interval_in_seconds = 30
    }
  }

  containers {
    display_name = "ollama"
    image_url    = "ollama/ollama:latest"
  }

  vnics {
    subnet_id             = var.subnet_id
    is_public_ip_assigned = true
  }
}

