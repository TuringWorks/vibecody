output "ip" { value = linode_instance.vibecody.ip_address }
output "url" { value = "http://${linode_instance.vibecody.ip_address}:7878" }
