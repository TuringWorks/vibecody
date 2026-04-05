terraform {
  required_providers {
    google = { source = "hashicorp/google"; version = "~> 5.0" }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

locals {
  tier_config = {
    lite = { cpu = "2", memory = "4Gi" }
    pro  = { cpu = "4", memory = "8Gi" }
    max  = { cpu = "8", memory = "16Gi" }
  }
  tier = local.tier_config[var.tier]
}

resource "google_cloud_run_v2_service" "vibecody" {
  name     = "vibecody"
  location = var.region

  template {
    containers {
      name  = "vibecli"
      image = var.image
      ports { container_port = 7878 }
      env { name = "VIBECLI_PROVIDER"; value = "ollama" }
      env { name = "OLLAMA_HOST"; value = "http://localhost:11434" }
      env { name = "RUST_LOG"; value = "info" }
      resources {
        limits = { cpu = local.tier.cpu; memory = local.tier.memory }
      }
      startup_probe { http_get { path = "/health"; port = 7878 }; initial_delay_seconds = 30 }
      liveness_probe { http_get { path = "/health"; port = 7878 }; period_seconds = 30 }
    }
    containers {
      name  = "ollama"
      image = "ollama/ollama:latest"
      ports { container_port = 11434 }
      resources {
        limits = { cpu = local.tier.cpu; memory = local.tier.memory }
      }
    }
    scaling { min_instance_count = 1; max_instance_count = 1 }
  }
}

resource "google_cloud_run_service_iam_member" "public" {
  location = google_cloud_run_v2_service.vibecody.location
  service  = google_cloud_run_v2_service.vibecody.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}
