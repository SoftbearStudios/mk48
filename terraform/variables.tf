// Constants
variable "availability_zone" {
	default = "us-east-1b"
	type = string
}

variable "region" {
	default = "us-east-1"
	type = string
}

variable "stage" {
	default = "prod"
	type = string
}

// TODO: Add mk48io.com
variable "domain" {
	default = "mk48.io"
	type = string
}

variable "server_slots" {
	default = 4
	type = number
}
