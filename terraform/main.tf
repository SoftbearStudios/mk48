module "game_terraform" {
    source = "../engine/game_terraform"

    name = "mk48"
    domain = "mk48.io"
    servers = {
        1 = "us-east"
        2 = "us-east"
        3 = "us-east"
        4 = "ap-west"
    }
    aws_region = var.aws_region
    linode_token = var.linode_token
}

// From env.
variable "linode_token" {
    type = string
}

variable "aws_region" {
    type = string
    default = "us-east-1"
}

terraform {
    /*
    1) Manually create S3 bucket with name and region matching below.
    2) Manually create DynamoDB table with name and region matching below and primary key LockID (string)
    */
    backend "s3" {
        profile = "terraform"
        bucket = "softbear-terraform"
        key    = "mk48.tfstate"
        dynamodb_table = "mk48_terraform" // For locking.
        region = "us-east-1"
    }
}

terraform {
    required_providers {
        linode = {
            source  = "linode/linode"
            # version = "1.20.2"
        }
    }
}

provider "linode" {
    token = var.linode_token
}

provider "aws" {
    profile = "terraform"
    region = var.aws_region
}

data "aws_caller_identity" "current" {}