resource "aws_placement_group" "main" {
	name     = "mk48-${var.stage}-servers"
	strategy = "spread"
}

resource "aws_autoscaling_group" "main" {
	name                      = "mk48-${var.stage}-servers"
	max_size                  = var.server_slots
	min_size                  = 1
	health_check_grace_period = 300
	health_check_type         = "EC2"
	desired_capacity          = 2
	force_delete              = true
	// Instances must mark themselves unprotected before they can be deleted
	protect_from_scale_in     = true
	placement_group           = aws_placement_group.main.id
	launch_configuration      = aws_launch_configuration.main.name
	vpc_zone_identifier       = [aws_subnet.main.id]

	tag {
		key = "Name"
		value = "mk48-${var.stage}-server"
		propagate_at_launch = true
	}

	timeouts {
		delete = "15m"
	}
}
