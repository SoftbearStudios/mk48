#!/bin/bash

echo "Security measures"

sed -i 's/PasswordAuthentication yes/PasswordAuthentication no/g' /etc/ssh/sshd_config && service ssh restart

<<comment
cat <<EOF > /etc/sysctl.conf
# Enable Spoof protection (reverse-path filter)
# Turn on Source Address Verification in all interfaces to
# prevent some spoofing attacks
net.ipv4.conf.default.rp_filter=1
net.ipv4.conf.all.rp_filter=1

# Enable TCP/IP SYN cookies
# See http://lwn.net/Articles/277146/
# Note: This may impact IPv6 TCP sessions too
net.ipv4.tcp_syncookies=1

# Do not accept ICMP redirects (prevent MITM attacks)
net.ipv4.conf.all.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0

# Do not send ICMP redirects (we are not a router)
net.ipv4.conf.all.send_redirects = 0

# Do not accept IP source route packets (we are not a router)
net.ipv4.conf.all.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0
EOF
comment

cat <<EOF > /etc/nftables.conf
#!/usr/sbin/nft -f

flush ruleset

table inet filter {
	# Garbage collected
	set ipv4 {
		type ipv4_addr
		size 16384
		flags dynamic
	}

	# Expiry based
	set ipv4_timeout {
		type ipv4_addr
		size 16384
		flags dynamic, timeout
	}

	# Garbage collected
	set ipv6 {
		type ipv6_addr;
		size 16384
		flags dynamic
	}

	# Expiry based
	set ipv6_timeout {
		type ipv6_addr;
		size 16384
		flags dynamic, timeout
	}

	chain inbound_ipv4 {
		# Allow ICMP pings (with a global limit)
		icmp type echo-request limit rate 5/second accept

		# Limit connections per source IP
		ct state new add @ipv4 { ip saddr ct count over 64 } counter reject

		# Limit connection rate per source IP
		# ct state new add @ipv4_timeout { ip saddr timeout 30s limit rate over 12/second burst 128 packets } counter drop

		# Limit packet rate per source IP
		# ct state { established, related } add @ipv4_timeout { ip saddr timeout 30s limit rate over 2048/second burst 4096 packets } counter drop
	}

	chain inbound_ipv6 {
		icmpv6 type { nd-neighbor-solicit, nd-router-advert, nd-neighbor-advert } accept

		# Allow ICMP pings (with a global limit)
		icmpv6 type echo-request limit rate 5/second accept

		# Limit connections per source IP
		ct state new add @ipv6 { ip6 saddr ct count over 64 } counter reject

		# Limit connection rate per source IP
		ct state new add @ipv6_timeout { ip6 saddr timeout 30s limit rate over 12/second burst 128 packets } counter drop

		# Limit packet rate per source IP
		ct state { established, related } add @ipv6_timeout { ip6 saddr timeout 30s limit rate over 2048/second burst 4096 packets } counter drop
	}

	chain inbound {
		# What follows this is a whitelist
		type filter hook input priority 0; policy drop;

		# Protocol-specific rules
		meta protocol vmap { ip : jump inbound_ipv4, ip6 : jump inbound_ipv6 }

		# Allow existing connections to continue, drop invalid packets
		ct state vmap { established : accept, related : accept, invalid : drop }

		# Allow loopback
		iifname lo accept

		# Allow SSH (with a global limit)
		tcp dport ssh ct count 32 accept

		# Allow HTTP (without a global limit)
		tcp dport { http, https } accept
	}

	chain forward {
		# We are not a router.
		type filter hook forward priority 0; policy drop;
	}
}
EOF

nft -f /etc/nftables.conf

echo "Updating"

apt update

echo "Installing snap"

apt install -y snapd
snap install core;
snap refresh core;

echo "Installing linode token"

printf "dns_linode_key = $LINODE_TOKEN\ndns_linode_version = 4\n" > /root/linode.ini
chmod 600 /root/linode.ini

echo "Installing certbot"

snap install --classic certbot
ln -s /snap/bin/certbot /usr/bin/certbot
snap set certbot trust-plugin-with-root=ok
snap install certbot-dns-linode

printf "certbot certonly --agree-tos --non-interactive --dns-linode --dns-linode-credentials /root/linode.ini --dns-linode-propagation-seconds 180 --no-eff-email --no-redirect --email finnbearone@gmail.com -d $DOMAIN -d www.$DOMAIN -d $SERVER_ID.$DOMAIN" > get_ssl_cert.sh
chmod u+x /root/get_ssl_cert.sh
./get_ssl_cert.sh

#echo "Creating server download script..."
#printf "aws s3 cp s3://${aws_s3_bucket.static.bucket}/server /root/game-server\n\nchmod u+x /root/server" > /root/download-game-server.sh
#chmod u+x /root/download-game-server.sh

#echo "Downloading server..."
#/root/download-game-server.sh

echo "Installing service..."
cat <<EOF > /etc/systemd/system/game-server.service
[Unit]
Description=Game Server

[Service]
Type=simple
User=root
Group=root
Restart=always
RestartSec=3
EnvironmentFile=/etc/environment
WorkingDirectory=~
ExecStart=/root/server \
  --server-id $SERVER_ID \
  --ip-address $IP_ADDRESS \
  --domain $DOMAIN \
  --chat-log /root/chat.log \
  --trace-log /root/trace.log \
  --admin-config-file /root/admin.toml \
  --discord-client-secret $DISCORD_CLIENT_SECRET \
  --discord-bot-token $DISCORD_BOT_TOKEN \
  --linode-personal-access-token $LINODE_TOKEN \
  --certificate-path /etc/letsencrypt/live/$DOMAIN/fullchain.pem \
  --private-key-path /etc/letsencrypt/live/$DOMAIN/privkey.pem

[Install]
WantedBy=multi-user.target
EOF

echo "Enabling service..."
sudo systemctl daemon-reload
sudo systemctl start game-server
sudo systemctl enable game-server

echo "Installing util scripts..."
printf "journalctl -a -f -o cat -u game-server" > /root/view-game-server-logs.sh
chmod u+x /root/view-game-server-logs.sh

printf "sudo systemctl restart game-server" > /root/restart-game-server.sh
chmod u+x /root/restart-game-server.sh

#printf "/root/download-game-server.sh\n/root/restart-game-server.sh\n/root/view-game-server-logs.sh" > /root/update-game-server.sh
#chmod u+x /root/update-game-server.sh

printf "journalctl -a --no-pager -o cat -u game-server | grep -i \$1" > /root/grep-game-server-logs.sh
chmod u+x /root/grep-game-server-logs.sh

echo "Init done."