output "ip" { value = digitalocean_droplet.vibecody.ipv4_address }
output "url" { value = "http://${digitalocean_droplet.vibecody.ipv4_address}:7878" }
