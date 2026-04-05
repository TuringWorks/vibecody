variable "do_token" { description = "DigitalOcean API token"; type = string; sensitive = true }
variable "region" { type = string; default = "nyc3" }
variable "tier" { type = string; default = "lite"; validation { condition = contains(["lite", "pro", "max"], var.tier); error_message = "Must be lite, pro, or max." } }
variable "ssh_key_ids" { description = "SSH key fingerprints"; type = list(string); default = [] }
variable "image" { type = string; default = "ghcr.io/turingworks/vibecody:latest" }
