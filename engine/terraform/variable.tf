variable "name" {
  type = string
  default = "core"
}

variable "aws_region" {
  type = string
  default = "us-east-1"
}

// From env.
variable "linode_token" {
  type = string
}