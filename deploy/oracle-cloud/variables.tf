variable "compartment_id" { description = "OCI compartment OCID"; type = string }
variable "availability_domain" { description = "Availability domain"; type = string }
variable "subnet_id" { description = "Subnet OCID"; type = string }
variable "region" { description = "OCI region"; type = string; default = "us-ashburn-1" }
variable "tier" { description = "Resource tier"; type = string; default = "lite"; validation { condition = contains(["lite", "pro", "max"], var.tier); error_message = "Tier must be lite, pro, or max." } }
variable "image" { description = "Container image"; type = string; default = "ghcr.io/turingworks/vibecody:latest" }
