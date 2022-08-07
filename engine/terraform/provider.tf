terraform {
  required_providers {
    linode = {
      source  = "linode/linode"
      # version = "1.20.2"
    }
  }
}

terraform {
  /*
  1) Manually create S3 bucket with name and region matching below.
  2) Manually create DynamoDB table with name and region matching below and primary key LockID (string)
  */
  backend "s3" {
    profile = "terraform"
    bucket = "softbear-terraform"
    key    = "core.tfstate"
    dynamodb_table = "core_terraform" // For locking.
    region = "us-east-1"
  }
}

provider "aws" {
  profile = "terraform"
  region = var.aws_region
}

data "aws_caller_identity" "current" {}

provider "linode" {
  token = var.linode_token
}
