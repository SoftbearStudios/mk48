locals {
	default_cache_control = "no-transform, public, max-age=600"
}

resource "aws_s3_bucket" "www" {
	acl = "private"
	bucket = "mk48-${var.stage}-www"
	cors_rule {
		allowed_headers = ["*"]
		allowed_methods = ["GET"]
		allowed_origins = ["*"]
	}
	website {
		redirect_all_requests_to = "https://${var.domain}"
	}
}

data "aws_iam_policy_document" "www" {
	statement {
		sid = "cloudfront"
		actions = ["s3:GetObject"]
		resources = ["${aws_s3_bucket.www.arn}/*"]

		principals {
			type = "AWS"
			identifiers = [aws_cloudfront_origin_access_identity.main.iam_arn]
		}
	}
}

resource "aws_s3_bucket_policy" "www" {
	depends_on = [aws_cloudfront_origin_access_identity.main]
	bucket = aws_s3_bucket.www.id
	policy = data.aws_iam_policy_document.www.json
}

resource "aws_s3_bucket_public_access_block" "www" {
	bucket = aws_s3_bucket.www.id
	block_public_acls     = true
	block_public_policy = true
	ignore_public_acls = true
	restrict_public_buckets = true
}

resource "aws_s3_bucket" "static" {
	acl = "private"
	bucket = "mk48-${var.stage}-static"
	cors_rule {
		allowed_headers = ["*"]
		allowed_methods = ["GET"]
		allowed_origins = ["https://${var.domain}", "https://www.${var.domain}"]
	}
	website {
		index_document = "index.html"
		error_document = "404.html"

		routing_rules = jsonencode(
			[
				{
					Condition = {
						KeyPrefixEquals = "server"
					}
					Redirect  = {
						ReplaceKeyWith = "index.html"
					}
				},
			]
		)
	}
}

data "aws_iam_policy_document" "static" {
	statement {
		sid = "cloudfront"
		actions = ["s3:GetObject"]
		resources = ["${aws_s3_bucket.static.arn}/*"]

		principals {
			type = "*"
			identifiers = ["*"]
		}
	}
}

resource "aws_s3_bucket_policy" "static" {
	depends_on = [aws_cloudfront_origin_access_identity.main]
	bucket = aws_s3_bucket.static.id
	policy = data.aws_iam_policy_document.static.json
}

resource "aws_s3_bucket_public_access_block" "static" {
	bucket = aws_s3_bucket.static.id
	block_public_acls     = false
	block_public_policy = false
	ignore_public_acls = false
	restrict_public_buckets = false
}

resource "aws_s3_bucket_object" "svelte" {
	bucket = aws_s3_bucket.static.bucket
	cache_control = "no-cache"
	content_type = {
		"html" = "text/html",
		"xml" = "application/xml",
		"txt" = "text/plain",
		"js" = "application/javascript",
		"wasm" = "application/wasm",
		"css" = "text/css",
		"json" = "application/json",
		"ico" = "image/x-icon",
		"png" = "image/png",
		"webp" = "image/webp",
	}[split(".", each.value)[length(split(".", each.value)) - 1]]
	content_encoding = {
		"html" = null,
		"xml" = null,
		"txt" = null,
		"js" = null,
		"wasm" = "gzip",
		"css" = null,
		"json" = null,
		"ico" = null,
		"png" = null,
		"webp" = null,
	}[split(".", each.value)[length(split(".", each.value)) - 1]]
	etag = filemd5("../client/build/${each.value}")
	for_each = fileset("../client/build", "**/*.*")
	key = each.value
	source = "../client/build/${each.value}"
}

resource "aws_s3_bucket_object" "server" {
	bucket = aws_s3_bucket.static.bucket
	cache_control = "no-cache"
	content_type = "binary/octet-stream"
	etag = filemd5("../server/server")
	key = "server"
	source = "../server/server"
}
