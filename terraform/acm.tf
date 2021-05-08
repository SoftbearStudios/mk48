resource "aws_acm_certificate" "main" {
	domain_name = var.domain
	lifecycle {
		create_before_destroy = true
	}
	subject_alternative_names = [
		"*.${var.domain}"
	]
	validation_method = "DNS"
}

resource "aws_acm_certificate_validation" "main" {
	certificate_arn = aws_acm_certificate.main.arn
	validation_record_fqdns = [for record in aws_route53_record.cert: record.fqdn]
}
