SSH_KEY := ../.ssh/id_rsa
SSH_OPTS := -i $(SSH_KEY) -o "UserKnownHostsFile=/dev/null" -o "StrictHostKeyChecking=no"
SCP_OPTS := $(SSH_OPTS) -C


.PHONY: all debug_core target/release/server

all: debug_core

debug_core:
	RUST_BACKTRACE=1 cargo run -- --debug-core debug --max-bots 0 --database-read-only --http-port 8081

debug_bots:
	RUST_BACKTRACE=1 cargo run --release -- --min-bots 50000 --database-read-only

target/release/server:
	#RUSTFLAGS="-Ctarget-feature=-retpoline,+mmx,+aes,+sse,+sse2,+sse3,+sse4.1,+sse4.2,+popcnt" cargo build --release
	cargo build --release

ssh:
	ssh $(SSH_OPTS) root@mk48.io

new1: target/release/server
	scp $(SCP_OPTS) target/release/server root@45.79.190.29:/root/server1
	ssh $(SSH_OPTS) root@45.79.190.29 'mv /root/server1 /root/server && ./restart-game-server.sh'

new2: target/release/server
	scp $(SCP_OPTS) target/release/server root@173.255.228.145:/root/server1
	ssh $(SSH_OPTS) root@173.255.228.145 'mv /root/server1 /root/server && ./restart-game-server.sh'

new3: target/release/server
	scp $(SCP_OPTS) target/release/server root@173.255.225.233:/root/server1
	ssh $(SSH_OPTS) root@173.255.225.233 'mv /root/server1 /root/server && ./restart-game-server.sh'

new4: target/release/server
	scp $(SCP_OPTS) target/release/server root@194.195.115.239:/root/server1
	ssh $(SSH_OPTS) root@194.195.115.239 'mv /root/server1 /root/server && ./restart-game-server.sh'

client1:
	cargo run --release --manifest-path ../engine/upload_game_client/Cargo.toml -- --path ../js/public --url https://1.mk48.io/admin

client4:
	cargo run --release --manifest-path ../engine/upload_game_client/Cargo.toml -- --path ../js/public --url https://4.mk48.io/admin

ssh2:
	ssh $(SSH_OPTS) root@96.126.107.185

