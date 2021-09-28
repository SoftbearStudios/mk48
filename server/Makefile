DEV_SERVER_0 = root@192.155.88.93
DEV_SERVER_1 = root@97.107.139.198
SSH_KEY := ../.ssh/id_rsa
SSH_OPTS := -i $(SSH_KEY) -o "UserKnownHostsFile=/dev/null" -o "StrictHostKeyChecking=no"


.PHONY: all debug_core

all: debug_core

debug_core:
	RUST_BACKTRACE=1 cargo run -- --debug-core -v -v -p 0

ssh:
	ssh $(DEV_SERVER)

dev0: target/release/server
	cargo build --release
	scp $(SSH_OPTS) target/release/server $(DEV_SERVER_0):/root/server1
	ssh $(SSH_OPTS) $(DEV_SERVER_0) 'mv /root/server1 /root/server && ./restart-game-server.sh'

dev1: target/release/server
	cargo build --release
	scp $(SSH_OPTS) target/release/server $(DEV_SERVER_1):/root/server1
	ssh $(SSH_OPTS) $(DEV_SERVER_1) 'mv /root/server1 /root/server && ./restart-game-server.sh'
