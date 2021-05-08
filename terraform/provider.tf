provider "aws" {
	profile = "mk48"
	region = var.region
}

data "aws_caller_identity" "current" {}

provider "http" {}

/*
1) Manually create S3 bucket with name and region matching below.
2) Manually create DynamoDB table with name and region matching below and primary key LockID (string)
*/
terraform {
	backend "s3" {
		profile = "mk48"
		bucket = "mk48-terraform"
		key    = "terraform.tfstate"
		dynamodb_table = "mk48-terraform" // For locking
		region = "us-east-1"
	}
}
