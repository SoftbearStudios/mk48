# server.mk
#
# SPDX-FileCopyrightText: 2024 Softbear, Inc.
# SPDX-License-Identifier: LGPL-3.0-or-later
#

.PHONY: debug deploy release run run_debug run_release

debug:
	cargo build

deploy: release
ifdef GAME_ID
	aws s3 cp target/x86_64-unknown-linux-musl/release/server s3://plasma-prod-binaries/$(GAME_ID) --profile terraform
endif

network_latency: network_repair
	# one-way latency, jitter, correlation, loss, corruption
	sudo tc qdisc add dev lo root netem delay 100ms 10ms 25% loss 2% corrupt 0%

network_repair:
	-sudo tc qdisc delete dev lo root

release:
	cargo build --release --target x86_64-unknown-linux-musl

run: run_release

run_debug:
	cargo run -- --client-authenticate-burst 999999 --http-bandwidth-limit 99999999

run_debug_cpu_profile:
	cargo run -- --cpu-profile

run_release:
	cargo run --release --target x86_64-unknown-linux-musl -- --client-authenticate-burst 999999 --http-bandwidth-limit 99999999
