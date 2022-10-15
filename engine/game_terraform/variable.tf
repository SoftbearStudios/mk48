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

variable "discord_client_secret" {
  type = string
  sensitive = true
}

variable "discord_bot_token" {
  type = string
  sensitive = true
}

variable "linode_token" {
  type = string
  sensitive = true
}