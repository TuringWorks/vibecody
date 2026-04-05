variable "project_id" { description = "GCP project ID"; type = string }
variable "region" { description = "GCP region"; type = string; default = "us-central1" }
variable "tier" { description = "Resource tier"; type = string; default = "lite"; validation { condition = contains(["lite", "pro", "max"], var.tier); error_message = "Tier must be lite, pro, or max." } }
variable "image" { description = "Container image"; type = string; default = "ghcr.io/turingworks/vibecody:latest" }
