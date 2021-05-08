resource "aws_route53_zone" "main" {
	name = var.domain
}

resource "aws_route53_record" "cloudfront" {
	zone_id = aws_route53_zone.main.zone_id
	name = aws_route53_zone.main.name
	type = "A"
	alias {
		name = aws_cloudfront_distribution.main.domain_name
		zone_id = aws_cloudfront_distribution.main.hosted_zone_id
		evaluate_target_health = false
	}
}

resource "aws_route53_record" "cloudfront_www" {
	zone_id = aws_route53_zone.main.zone_id
	name = "www.${aws_route53_zone.main.name}"
	type = "A"
	alias {
		name = aws_cloudfront_distribution.www.domain_name
		zone_id = aws_cloudfront_distribution.www.hosted_zone_id
		evaluate_target_health = false
	}
}

resource "aws_route53_record" "cloudfront_servers" {
	count = var.server_slots
	zone_id = aws_route53_zone.main.zone_id
	name = "cf-${var.region}-${count.index}.${aws_route53_zone.main.name}"
	type = "A"
	alias {
		name = aws_cloudfront_distribution.servers[count.index].domain_name
		zone_id = aws_cloudfront_distribution.servers[count.index].hosted_zone_id
		evaluate_target_health = false
	}
}

# from https://github.com/terraform-providers/terraform-provider-aws/issues/10098#issuecomment-663562342
resource "aws_route53_record" "cert" {
  for_each = {
	for dvo in aws_acm_certificate.main.domain_validation_options: dvo.domain_name => {
	  name   = dvo.resource_record_name
	  record = dvo.resource_record_value
	  type   = dvo.resource_record_type
	}
  }
  allow_overwrite = true # may not be needed
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 900
  type            = each.value.type
  zone_id         = aws_route53_zone.main.zone_id
}
