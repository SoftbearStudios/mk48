variable "name" {
  type = string
  default = "mk48"
}

variable "domain" {
  type = string
  default = "mk48.io"
}

variable "region" {
  type = string
  default = "us-east"
}

variable "aws_region" {
  type = string
  default = "us-east-1"
}

variable "servers" {
  type = number
  default = 2
}

variable "linode_token" {
  type = string
}