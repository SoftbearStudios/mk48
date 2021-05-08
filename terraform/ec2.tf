#
# Copyright (c) 2020 by Softbear, Inc.
#

resource "aws_key_pair" "main" {
	key_name = "mk48-${var.stage}"
	public_key = file("../.ssh/id_rsa.pub")
}

resource "aws_iam_role_policy" "server" {
	name = "mk48-main-policy"
	role = aws_iam_role.server.name

	policy = <<EOF
{
	"Version": "2012-10-17",
	"Statement": [
		{
			"Sid": "s3",
			"Action": "s3:*",
			"Effect": "Allow",
			"Resource": [
				"${aws_s3_bucket.static.arn}",
				"${aws_s3_bucket.static.arn}/*"
			]
		},
		{
			"Sid": "route53",
			"Effect": "Allow",
			"Action": "route53:*",
			"Resource": [
				"arn:aws:route53:::hostedzone/${aws_route53_zone.main.zone_id}"
			]
		},
		{
			"Sid": "dynamodb",
			"Effect": "Allow",
			"Action": "dynamodb:*",
			"Resource": [
				"${aws_dynamodb_table.scores.arn}",
				"${aws_dynamodb_table.servers.arn}"
			]
		},
		{
			"Sid": "autoscaling",
			"Action": "autoscaling:*",
			"Effect": "Allow",
			"Resource": "${aws_autoscaling_group.main.arn}"
		},
		{
			"Sid": "autoscaling2",
			"Action": "autoscaling:DescribeAutoScalingGroups",
			"Effect": "Allow",
			"Resource": "*"
		}
	]
}
EOF
}

resource "aws_iam_role" "server" {
	name = "mk48-${var.stage}-server"
	assume_role_policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
	  {
		  "Action": "sts:AssumeRole",
		  "Principal": {
			 "Service": "ec2.amazonaws.com"
		  },
		  "Effect": "Allow",
		  "Sid": ""
	  }
  ]
}
EOF
}

resource "aws_vpc" "main" {
	cidr_block       = "192.168.150.0/24"
	tags = {
		Name = "mk48-${var.stage}-vpc"
	}
}

resource "aws_subnet" "main" {
	availability_zone = var.availability_zone
	cidr_block = "192.168.150.0/24"
	vpc_id = aws_vpc.main.id
	tags = {
		Name = "mk48-${var.stage}-subnet"
	}
}

resource "aws_internet_gateway" "main" {
	tags = {
		Name = "mk48-${var.stage}-gateway"
	}
	vpc_id = aws_vpc.main.id
}

resource "aws_route_table" "main" {
	vpc_id = aws_vpc.main.id
	route {
		cidr_block = "0.0.0.0/0"
		gateway_id = aws_internet_gateway.main.id
	}
	tags = {
		Name = "mk48-${var.stage}-routes"
	}
}

resource "aws_route_table_association" "main" {
	subnet_id = aws_subnet.main.id
	route_table_id = aws_route_table.main.id
}

resource "aws_network_interface" "main" {
	subnet_id   = aws_subnet.main.id
	private_ips = ["192.168.150.100"]
}

resource "aws_security_group" "main" {
	name         = "mk48-${var.stage}-security-group"
	description  = "Allow inbound mk48 ${var.stage} SSH and Web traffic"
	egress {
		from_port        = 0
		to_port          = 0
		protocol         = "-1"
		cidr_blocks      = ["0.0.0.0/0"]
	}
	ingress {
		from_port    = 22
		to_port      = 22
		protocol     = "tcp"
		cidr_blocks  = ["0.0.0.0/0"]
	}
	ingress {
		from_port    = 80
		to_port      = 80
		protocol     = "tcp"
		cidr_blocks  = ["0.0.0.0/0"]
	}
	ingress {
		from_port    = 8192
		to_port      = 8192
		protocol     = "tcp"
		cidr_blocks  = ["0.0.0.0/0"]
	}
	ingress {
		from_port    = 443
		to_port      = 443
		protocol     = "tcp"
		cidr_blocks  = ["0.0.0.0/0"]
	}
	vpc_id = aws_vpc.main.id
}

resource "aws_iam_instance_profile" "main" {
	name = "mk48-${var.stage}-profile"
	role = aws_iam_role.server.name
}

resource "aws_launch_configuration" "main" {
	// Using a prefix and the below lifecycle rule allows config
	// to be edited without destroying the autoscaling group
	name_prefix   = "mk48-${var.stage}-server-"
	lifecycle {
		create_before_destroy = true
	}
	image_id      = "ami-0a887e401f7654935"
	instance_type = "t3a.nano"
	key_name = aws_key_pair.main.key_name
	iam_instance_profile = aws_iam_instance_profile.main.name
	associate_public_ip_address = true
	root_block_device {
		encrypted = true
		volume_size = "8"
		volume_type = "gp2"
	}
	user_data = <<EOF
#!
echo "Creating server download script..."
printf "aws s3 cp s3://${aws_s3_bucket.static.bucket}/server /home/ec2-user/mk48-server\nchown ec2-user:ec2-user /home/ec2-user/mk48-server\nchmod u+x /home/ec2-user/mk48-server" > /home/ec2-user/download-mk48-server.sh
chown ec2-user:ec2-user /home/ec2-user/download-mk48-server.sh
chmod u+x /home/ec2-user/download-mk48-server.sh

echo "Downloading server..."
/home/ec2-user/download-mk48-server.sh

echo "Installing service..."
printf "${file("../server/mk48-server.service")}" > /etc/systemd/system/mk48-server.service

echo "Enabling service..."
sudo systemctl daemon-reload
sudo systemctl start mk48-server
sudo systemctl enable mk48-server

echo "Installing util scripts..."
printf "journalctl -a -f -o cat -u mk48-server" > /home/ec2-user/view-mk48-server-logs.sh
chown ec2-user:ec2-user /home/ec2-user/view-mk48-server-logs.sh
chmod u+x /home/ec2-user/view-mk48-server-logs.sh

printf "sudo systemctl restart mk48-server" > /home/ec2-user/restart-mk48-server.sh
chown ec2-user:ec2-user /home/ec2-user/restart-mk48-server.sh
chmod u+x /home/ec2-user/restart-mk48-server.sh

printf "/home/ec2-user/download-mk48-server.sh\n/home/ec2-user/restart-mk48-server.sh\n/home/ec2-user/view-mk48-server-logs.sh" > /home/ec2-user/update-mk48-server.sh
chown ec2-user:ec2-user /home/ec2-user/update-mk48-server.sh
chmod u+x /home/ec2-user/update-mk48-server.sh

printf "journalctl -a --no-pager -o cat -u mk48-server | grep -i \$1" > /home/ec2-user/grep-mk48-server-logs.sh
chown ec2-user:ec2-user /home/ec2-user/grep-mk48-server-logs.sh
chmod u+x /home/ec2-user/grep-mk48-server-logs.sh

echo "User data shell script done."

# Variables (denoted by equals sign)
DOMAIN="${var.domain}"
REGION="${var.region}"
STAGE="${var.stage}"
ROUTE53_ZONEID="${aws_route53_zone.main.zone_id}"
SERVER_SLOTS=${var.server_slots}
EOF
	security_groups = [
		aws_security_group.main.id
	]
}
