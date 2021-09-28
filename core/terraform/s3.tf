resource "aws_s3_bucket" "private" {
  acl = "private"
  bucket = "${var.name}-private-bucket"
}

resource "aws_s3_bucket_public_access_block" "private" {
  bucket = aws_s3_bucket.private.id
  block_public_acls = true
  block_public_policy = true
  ignore_public_acls = true
  restrict_public_buckets = true
}

output "private_s3_bucket" {
  value = aws_s3_bucket.private.bucket_regional_domain_name
}