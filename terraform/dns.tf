resource "linode_domain" "main" {
  type = "master"
  domain = var.domain
  soa_email = "finnbearone@gmail.com"
  tags = [var.name]
}

// This is accomplished at runtime by the servutil watchdog.
/*
resource "linode_domain_record" "home_ipv4" {
  count = var.servers
  domain_id = linode_domain.main.id
  name = ""
  record_type = "A"
  target = element(linode_instance.servers.*.ip_address, count.index)
  ttl_sec = 30
}
*/

resource "linode_domain_record" "servers_ipv4" {
  count = var.servers
  domain_id = linode_domain.main.id
  name = count.index + 1
  record_type = "A"
  target = element(linode_instance.servers.*.ip_address, count.index)
  ttl_sec = 30
}