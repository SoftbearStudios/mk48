resource "aws_iam_access_key" "servers" {
  user    = aws_iam_user.servers.name
}

resource "aws_iam_user" "servers" {
  name = "${var.name}_servers"
  path = "/system/"
}

resource "aws_iam_user_policy" "servers" {
  name = "${var.name}_servers"
  user = aws_iam_user.servers.name

  policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "dynamodb",
      "Action": [
        "dynamodb:*"
      ],
      "Effect": "Allow",
      "Resource": [
        "${aws_dynamodb_table.logins.arn}",
        "${aws_dynamodb_table.metrics.arn}",
        "${aws_dynamodb_table.sessions.arn}",
        "${aws_dynamodb_table.scores.arn}",
        "${aws_dynamodb_table.users.arn}"
      ]
    },
    {
      "Sid": "s3",
      "Action": [
        "s3:*"
      ],
      "Effect": "Allow",
      "Resource": [
        "${aws_s3_bucket.private.arn}",
        "${aws_s3_bucket.private.arn}/*"
      ]
    }
  ]
}
EOF
}

output "aws_access_key_id" {
  value = aws_iam_access_key.servers.id
}

output "aws_secret_access_key" {
  value = aws_iam_access_key.servers.secret
  sensitive = true
}