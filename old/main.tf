terraform {
  backend "s3" {
    bucket  = "bealo-terraform-state"
    key     = "terraform.tfstate"
    region  = "eu-west-1"
    profile = "bealo"
  }
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.16"
    }
  }

  required_version = ">= 1.2.0"
}

provider "aws" {
  region  = "eu-west-1"
  profile = "bealo"
}

resource "aws_s3_bucket" "episode-storage" {
  bucket        = "tommie-the-brain-in-space-media"
  force_destroy = true
}

# resource "aws_instance" "app_server" {
#   ami           = "ami-01f5f2e96f603b15b"
#   instance_type = "t2.micro"
# 
#   tags = {
#     Name = "ExampleAppServerInstance"
#   }
# }
