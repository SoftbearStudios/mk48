resource "linode_firewall" "game_server" {
  label = "game_server"
  tags = []

  inbound {
    label    = "HTTP_SSH"
    action = "ACCEPT"
    protocol  = "TCP"
    ports     = "443,80,22"
    ipv4 = ["0.0.0.0/0"]
    ipv6 = ["::/0"]
  }

  inbound {
    label    = "ICMP"
    action = "ACCEPT"
    protocol  = "ICMP"
    ipv4 = ["0.0.0.0/0"]
    ipv6 = ["::/0"]
  }

  inbound_policy = "DROP"
  outbound_policy = "ACCEPT"
}

output "game_server_firewall_id" {
  value = linode_firewall.game_server.id
}