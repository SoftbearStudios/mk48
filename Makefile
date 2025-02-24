include makefiles/game.mk

server_perf_record:
	cargo build --manifest-path server/Cargo.toml
	sudo perf record --call-graph=dwarf ./server/target/release/server

server_perf_report:
	sudo perf report --hierarchy -f -M intel

../mk48_publish:
	git clone git@github.com:SoftbearStudios/mk48.git ../mk48_publish

publish_reset:
	rm -rf ../mk48_publish/*

publish_copy:
	rsync -av --progress * ../mk48_publish/ --exclude ./Makefile --exclude server_fuzzer --exclude server/perf.data --exclude server/perf.data.old --exclude client_static --exclude target --exclude yew --exclude .git --exclude .terraform --exclude terraform --exclude engine/game_terraform --exclude engine/terraform --exclude .terraform.lock.hcl --exclude terrain_test --exclude node_modules --exclude .ssh --exclude submodules.sh --exclude serde-test
	rm ../mk48_publish/Makefile
	cp .gitignore ../mk48_publish/
	cp -r .cargo ../mk48_publish/
	cp -r .github ../mk48_publish/

publish: publish_reset publish_copy
