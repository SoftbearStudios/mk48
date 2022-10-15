resource "linode_domain" "main" {
  type = "master"
  domain = var.domain
  refresh_sec = 300
  retry_sec = 30
  expire_sec = 604800
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
  for_each = var.servers
  domain_id = linode_domain.main.id
  name = each.key
  record_type = "A"
  target = linode_instance.servers[each.key].ip_address
  ttl_sec = 120
}

resource "linode_domain_record" "www" {
  domain_id = linode_domain.main.id
  name = "www"
  record_type = "CNAME"
  target = var.domain
  ttl_sec = 120
}