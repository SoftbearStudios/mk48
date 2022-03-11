resource "linode_instance" "servers_new" {
    depends_on = [linode_domain.main]
    for_each = var.servers
    label = "${var.name}_${each.key}"
    image = "linode/debian11"
    region = each.value
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
            "echo \"SERVER_ID=\\\"${each.key}\\\"\" >> /etc/environment",
            "echo \"IP_ADDRESS=\\\"${self.ip_address}\\\"\" >> /etc/environment",
            "echo \"DOMAIN_HOME=\\\"${var.domain}\\\"\" >> /etc/environment",
            "echo \"DOMAIN=\\\"${each.key}.${var.domain}\\\"\" >> /etc/environment",
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