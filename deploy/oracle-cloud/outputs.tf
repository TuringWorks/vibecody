output "public_ip" {
  value       = oci_container_instances_container_instance.vibecody.vnics[0].private_ip
  description = "VibeCody public IP"
}
