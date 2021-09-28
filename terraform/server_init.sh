#!/bin/bash

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

printf "certbot certonly --agree-tos --non-interactive --dns-linode --dns-linode-credentials /root/linode.ini --dns-linode-propagation-seconds 120 --no-eff-email --no-redirect --email finnbearone@gmail.com -d $DOMAIN_HOME -d *.$DOMAIN_HOME" > get_ssl_cert.sh
chmod u+x /root/get_ssl_cert.sh
./get_ssl_cert.sh

#echo "Creating server download script..."
#printf "aws s3 cp s3://${aws_s3_bucket.static.bucket}/server /root/game-server\n\nchmod u+x /root/server" > /root/download-game-server.sh
#chmod u+x /root/download-game-server.sh

#echo "Downloading server..."
#/root/download-game-server.sh

echo "Installing service..."
printf "[Unit]\nDescription=Game Server\n[Service]\nType=simple\nUser=root\nGroup=root\nRestart=always\nRestartSec=3\nEnvironmentFile=/etc/environment\nExecStart=/root/server -v -v --debug-game --chat-log /root/chat.log --certificate-path /etc/letsencrypt/live/$DOMAIN_HOME/fullchain.pem --private-key-path /etc/letsencrypt/live/$DOMAIN_HOME/privkey.pem\n[Install]\nWantedBy=multi-user.target" > /etc/systemd/system/game-server.service

echo "Enabling service..."
sudo systemctl daemon-reload
sudo systemctl start game-server
sudo systemctl enable game-server

echo "Installing util scripts..."
printf "journalctl -a -f -o cat -u game-server" > /root/view-game-server-logs.sh
chmod u+x /root/view-game-server-logs.sh

printf "sudo systemctl restart game-server" > /root/restart-game-server.sh
chmod u+x /root/restart-game-server.sh

printf "/root/download-game-server.sh\n/root/restart-game-server.sh\n/root/view-game-server-logs.sh" > /root/update-game-server.sh
chmod u+x /root/update-game-server.sh

printf "journalctl -a --no-pager -o cat -u game-server | grep -i \$1" > /root/grep-game-server-logs.sh
chmod u+x /root/grep-game-server-logs.sh

echo "Init done."