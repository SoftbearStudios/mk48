SSH_KEY := ../.ssh/id_rsa
SSH_OPTS := -i $(SSH_KEY) -o "UserKnownHostsFile=/dev/null" -o "StrictHostKeyChecking=no"
SCP_OPTS := $(SSH_OPTS) -C


.PHONY: all debug_core

all: debug_core

debug_core:
	RUST_BACKTRACE=1 cargo run -- --debug-core debug -p 0 --database-read-only

ssh:
	ssh $(SSH_OPTS) root@mk48.io

new1: target/release/server
	cargo build --release
	scp $(SCP_OPTS) target/release/server root@45.56.106.218:/root/server1
	ssh $(SSH_OPTS) root@45.56.106.218 'mv /root/server1 /root/server && ./restart-game-server.sh'

new2: target/release/server
	cargo build --release
	scp $(SCP_OPTS) target/release/server root@96.126.107.185:/root/server1
	ssh $(SSH_OPTS) root@96.126.107.185 'mv /root/server1 /root/server && ./restart-game-server.sh'

ssh2:
	ssh $(SSH_OPTS) root@96.126.107.185
