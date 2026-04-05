variable "linode_token" { type = string; sensitive = true }
variable "region" { type = string; default = "us-east" }
variable "tier" { type = string; default = "lite"; validation { condition = contains(["lite", "pro", "max"], var.tier); error_message = "Must be lite, pro, or max." } }
variable "root_password" { type = string; sensitive = true }
