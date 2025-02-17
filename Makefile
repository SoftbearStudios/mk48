include makefiles/game.mk

server_perf_record:
	cargo build --manifest-path server/Cargo.toml
	sudo perf record --call-graph=dwarf ./server/target/release/server

server_perf_report:
	sudo perf report --hierarchy -f -M intel
