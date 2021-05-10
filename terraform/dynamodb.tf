resource "aws_dynamodb_table" "scores" {
	name = "mk48-${var.stage}-scores"
	billing_mode = "PAY_PER_REQUEST"
	hash_key = "type"
	range_key = "name"

	attribute {
		name = "type"
		type = "S"
	}

	attribute {
		name = "name"
		type = "S"
	}

	ttl {
		attribute_name = "ttl"
		enabled        = true
	}

	point_in_time_recovery {
		enabled = true
	}
}

resource "aws_dynamodb_table" "servers" {
	name = "mk48-${var.stage}-servers"
	billing_mode = "PAY_PER_REQUEST"
	hash_key = "region"
	range_key = "slot"

	attribute {
		name = "region"
		type = "S"
	}

	attribute {
		name = "slot"
		type = "N"
	}

	ttl {
		attribute_name = "ttl"
		enabled        = true
	}

	point_in_time_recovery {
		enabled = true
	}
}

resource "aws_dynamodb_table" "statistics" {
	name = "mk48-${var.stage}-statistics"
	billing_mode = "PAY_PER_REQUEST"
	hash_key = "region"
	range_key = "timestamp"

	attribute {
		name = "region"
		type = "S"
	}

	attribute {
		name = "timestamp"
		type = "N"
	}

	point_in_time_recovery {
		enabled = true
	}
}
