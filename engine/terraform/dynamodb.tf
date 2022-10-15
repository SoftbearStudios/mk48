resource "aws_dynamodb_table" "sessions" {
  name = "core_sessions"
  billing_mode = "PAY_PER_REQUEST"
  hash_key = "arena_id"
  range_key = "session_id"

  attribute {
    name = "arena_id"
    type = "N"
  }

  attribute {
    name = "session_id"
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

resource "aws_dynamodb_table" "scores" {
  name = "core_scores"
  billing_mode = "PAY_PER_REQUEST"
  hash_key = "game_id_score_type"
  range_key = "alias"

  attribute {
    name = "game_id_score_type"
    type = "S"
  }

  attribute {
    name = "alias"
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

resource "aws_dynamodb_table" "metrics" {
  name = "core_metrics"
  billing_mode = "PAY_PER_REQUEST"
  hash_key = "game_id"
  range_key = "timestamp"

  attribute {
    name = "game_id"
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

resource "aws_dynamodb_table" "users" {
  name = "core_users"
  billing_mode = "PAY_PER_REQUEST"
  hash_key = "user_id"

  attribute {
    name = "user_id"
    type = "N"
  }

  stream_enabled = true
  stream_view_type = "NEW_IMAGE"

  point_in_time_recovery {
    enabled = true
  }
}

resource "aws_dynamodb_table" "logins" {
  name = "core_logins"
  billing_mode = "PAY_PER_REQUEST"
  hash_key = "login_type"
  range_key = "id"

  attribute {
    name = "login_type"
    type = "S"
  }

  attribute {
    name = "id"
    type = "S"
  }

  attribute {
    name = "user_id"
    type = "N"
  }

  global_secondary_index {
    hash_key = "user_id"
    name = "user_id"
    projection_type = "ALL"
  }

  stream_enabled = true
  stream_view_type = "NEW_IMAGE"

  point_in_time_recovery {
    enabled = true
  }
}
