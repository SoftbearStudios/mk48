variable "name" {
  type = string
}

variable "domain" {
  type = string
}

variable "servers" {
  type = map
  default = {
    1 = "us-east"
    2 = "us-east"
  }
}

variable "aws_region" {
  type = string
}

variable "linode_token" {
  type = string
  sensitive = true
}