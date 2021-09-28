data "terraform_remote_state" "core" {
  backend = "s3"

  config = {
    profile = "terraform"
    bucket = "softbear-terraform"
    key    = "core.tfstate"
    dynamodb_table = "core_terraform" // For locking.
    region = "us-east-1"
  }
}