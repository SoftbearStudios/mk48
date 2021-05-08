locals {
	static_dist_origin_id = "S3-static"
	www_dist_origin_id = "S3-www"
	server_dist_origin_id = "EC2-server"
}

resource "aws_cloudfront_origin_access_identity" "main" {
	comment = "Mk48 ${var.stage}"
}

resource "aws_cloudfront_distribution" "main" {
	depends_on = [aws_acm_certificate_validation.main, aws_s3_bucket.static]
	aliases = [var.domain]
	default_root_object = "index.html"
	enabled = true
	is_ipv6_enabled = true
	price_class = "PriceClass_100"
	origin {
		domain_name = aws_s3_bucket.static.website_endpoint
		origin_id = local.static_dist_origin_id
		custom_origin_config {
			http_port = 80
			https_port = 443
			origin_protocol_policy = "http-only"
			origin_ssl_protocols = ["TLSv1.1", "TLSv1.2"]
		}
	}
	default_cache_behavior {
		allowed_methods = ["GET", "HEAD", "OPTIONS"]
		cached_methods = ["GET", "HEAD"]
		compress = true
		target_origin_id = local.static_dist_origin_id

		forwarded_values {
			query_string = false

			cookies {
				forward = "all"
			}
		}

		viewer_protocol_policy = "redirect-to-https"
	}
	viewer_certificate {
		acm_certificate_arn = aws_acm_certificate.main.arn
		minimum_protocol_version = "TLSv1.1_2016"
		ssl_support_method = "sni-only"
	}
	restrictions {
		geo_restriction {
			restriction_type = "none"
		}
	}
}

resource "aws_cloudfront_distribution" "www" {
  depends_on = [aws_acm_certificate_validation.main]
  aliases = ["www.${var.domain}"]
  default_root_object = ""
  enabled = true
  is_ipv6_enabled = true
  price_class = "PriceClass_100"
  origin {
	domain_name = aws_s3_bucket.www.website_endpoint
	origin_id = local.www_dist_origin_id
	custom_origin_config {
		http_port = 80
		https_port = 443
		origin_protocol_policy = "http-only"
		origin_ssl_protocols = ["TLSv1.1", "TLSv1.2"]
	}
  }
  default_cache_behavior {
	allowed_methods = ["GET", "HEAD", "OPTIONS"]
	cached_methods = ["GET", "HEAD"]
	compress = true
	target_origin_id = local.www_dist_origin_id

	forwarded_values {
	  query_string = false

	  cookies {
		forward = "all"
	  }
	}

	viewer_protocol_policy = "redirect-to-https"
  }
  viewer_certificate {
	acm_certificate_arn = aws_acm_certificate.main.arn
	minimum_protocol_version = "TLSv1.1_2016"
	ssl_support_method = "sni-only"
  }
  restrictions {
	geo_restriction {
	  restriction_type = "none"
	}
  }
}

resource "aws_cloudfront_distribution" "servers" {
	count = var.server_slots
	depends_on = [aws_acm_certificate_validation.main]
	aliases = ["cf-${var.region}-${count.index}.${var.domain}"]
	default_root_object = "index.html"
	enabled = true
	is_ipv6_enabled = true
	price_class = "PriceClass_100"
	origin {
		domain_name = "ws-${var.region}-${count.index}.${var.domain}"
		origin_id = local.server_dist_origin_id
		custom_origin_config {
			http_port = 8192
			https_port = 443
			origin_protocol_policy = "http-only"
			origin_ssl_protocols = ["TLSv1.2"]
		}
	}
	default_cache_behavior {
		allowed_methods = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"]
		cached_methods = ["GET", "HEAD"]
		target_origin_id = local.server_dist_origin_id

		forwarded_values {
			headers = ["*"]
			query_string = true

			cookies {
				forward = "all"
			}
		}

		viewer_protocol_policy = "redirect-to-https"
		min_ttl                = 0
		default_ttl            = 0
		max_ttl                = 3600
	}
	viewer_certificate {
		acm_certificate_arn = aws_acm_certificate.main.arn
		minimum_protocol_version = "TLSv1.1_2016"
		ssl_support_method = "sni-only"
	}
	restrictions {
		geo_restriction {
			restriction_type = "none"
		}
	}
}
