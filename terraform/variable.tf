variable "name" {
  type = string
  default = "mk48"
}

variable "domain" {
  type = string
  default = "mk48.io"
}

variable "servers" {
  type = map
  default = {
    1 = "us-east"
    2 = "us-east"
    3 = "us-east"
    4 = "ap-west"
  }
}

variable "aws_region" {
  type = string
  default = "us-east-1"
}

variable "linode_token" {
  type = string
}