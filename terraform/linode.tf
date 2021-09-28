resource "linode_instance" "servers" {
    depends_on = [linode_domain.main]
    count = var.servers
    label = "${var.name}_server_${count.index}"
    image = "linode/debian11"
    region = var.region
    type = "g6-nanode-1"
    authorized_keys = [
        "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC+TE0LBTlPK2g4ULX48WfBJZKk/8vs3/faGaEkr+Q8j6ZB3nl0qBVk7NI8ETxbqZ0WRXf21ExZUO6m+ecUB5JmkU19pw9zLwDB+TT8DVsjRDuMEW09afeMGux2eXOV+0w+G1qqqwH2V8zFGpRj91kNwvR2tZ5yc+r1NTC+T3gr5HeGXGb7Q82l7knUErSvCB52T0BR31lXT6FiNSdRt+uYAkAoe3gtdnlvKV3GkiWejgY3L6sXz63llnGDefxhXSATo6yj41UNbAXHxCHPmFNFktpYT+H2OkdRRdSSIcs+1/JtwEm3QKBkDsjKFrBP3ujuvlVOi1sStEesKyNAUOyX finnb@epyc",
        chomp(file("../.ssh/id_rsa.pub"))
    ]
    tags = [var.name]
    swap_size = 128
    private_ip = true
    backups_enabled = false

    connection {
        type     = "ssh"
        user     = "root"
        host     = self.ip_address
    }

    provisioner "file" {
        source      = "../server/target/release/server"
        destination = "/root/server"
    }

    provisioner "file" {
        source      = "./server_init.sh"
        destination = "/root/server_init.sh"
    }

    provisioner "remote-exec" {
        inline = [
            "chmod u+x /root/server",
            "chmod u+x /root/server_init.sh",
            "echo \"SERVER=\\\"${count.index}\\\"\" >> /etc/environment",
            "echo \"SERVER_COUNT=\\\"${var.servers}\\\"\" >> /etc/environment",
            "echo \"DOMAIN_HOME=\\\"${var.domain}\\\"\" >> /etc/environment",
            "echo \"DOMAIN=\\\"server${count.index}.${var.domain}\\\"\" >> /etc/environment",
            "echo \"AWS_ACCESS_KEY_ID=\\\"${data.terraform_remote_state.core.outputs.aws_access_key_id}\\\"\" >> /etc/environment",
            "echo \"AWS_SECRET_ACCESS_KEY=\\\"${data.terraform_remote_state.core.outputs.aws_secret_access_key}\\\"\" >> /etc/environment",
            "echo \"PRIVATE_S3_BUCKET=\\\"${data.terraform_remote_state.core.outputs.private_s3_bucket}\\\"\" >> /etc/environment",
            "echo \"LINODE_TOKEN=\\\"${var.linode_token}\\\"\" >> /etc/environment"
        ]
    }

    provisioner "remote-exec" {
        inline = [
            "/root/server_init.sh"
        ]
    }
}

/*
resource "linode_nodebalancer" "servers" {
    label = "${var.name}_servers"
    region = var.region
    client_conn_throttle = 20
    tags = [var.name]
}

resource "linode_nodebalancer_config" "main_80" {
    nodebalancer_id = linode_nodebalancer.servers.id
    port = 80
    protocol = "tcp"
    proxy_protocol = "v1"
    check = "connection"
    check_attempts = 3
    check_timeout = 30
    stickiness = "table"
    algorithm = "roundrobin"
}


resource "linode_nodebalancer_config" "main_443" {
    nodebalancer_id = linode_nodebalancer.servers.id
    port = 443
    protocol = "tcp"
    proxy_protocol = "v1"
    check = "connection"
    check_attempts = 3
    check_timeout = 30
    stickiness = "table"
    algorithm = "roundrobin"
}

resource "linode_nodebalancer_node" "servers_80" {
    count = var.servers
    nodebalancer_id = linode_nodebalancer.servers.id
    config_id = linode_nodebalancer_config.main_80.id
    address = "${element(linode_instance.servers.*.private_ip_address, count.index)}:80"
    label = "server_${count.index}"
}

resource "linode_nodebalancer_node" "servers_443" {
    count = var.servers
    nodebalancer_id = linode_nodebalancer.servers.id
    config_id = linode_nodebalancer_config.main_443.id
    address = "${element(linode_instance.servers.*.private_ip_address, count.index)}:443"
    label = "servers_${count.index}"
}
*/