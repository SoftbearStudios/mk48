SSH_OPTS = -i ../.ssh/id_rsa -o "UserKnownHostsFile=/dev/null" -o "StrictHostKeyChecking=no"
seconds=10
profile=profile
ZERO=ws-us-east-1-0.mk48.io
ONE=ws-us-east-1-1.mk48.io
TWO=ws-us-east-1-2.mk48.io
THREE=ws-us-east-1-3.mk48.io
.PHONY=run pprof pprof-server install upload_server

all: run

../client/src/data/entities.json: ../client/src/data/entities-raw.json
	( cd ../client/src/data; node preprocess; )

server: *.go */*.go */*/*.go ../client/src/data/entities.json
	cp ../client/src/data/entities.json ./world/entities.json
	go build -o server

server_arm: *.go world/*.go world/*/*.go ../client/src/data/entities.json
	cp ../client/src/data/entities.json ./world/entities.json
	GOARCH=arm64 go build -o server_arm

run: server
	./server $(s)

upload_server: server
	aws s3 cp ./server s3://mk48-prod-static/server --profile mk48 --cache-control "no-cache"

install-zero: upload_server
	@echo "Warning: will boot players off"
	@sleep 5
	ssh $(SSH_OPTS) ec2-user@$(ZERO) './update-mk48-server.sh'

install-one: upload_server
	ssh $(SSH_OPTS) ec2-user@$(ONE) './update-mk48-server.sh'

race:
	go build -o server_race -race && ./server_race $(s)

test: server
	go test
	go build -o server_test -race
	/usr/bin/test `GORACE="halt_on_error=1" timeout --preserve-status 60 ./server_test -players 100 -bot-max-spawn-level 10 > ./out.txt 2> ./err.txt; echo $$?;` -eq 143 || (cat out.txt err.txt; false)

pprof:
	go tool pprof -http localhost:8193 -seconds $(seconds) localhost:8192/debug/pprof/$(profile)

pprof-zero:
	go tool pprof -http localhost:8193 -seconds $(seconds) $(ZERO):8192/debug/pprof/$(profile)

pprof-one:
	go tool pprof -http localhost:8193 -seconds $(seconds) $(ONE):8192/debug/pprof/$(profile)

ssh-zero:
	ssh $(SSH_OPTS) ec2-user@$(ZERO)

ssh-one:
	ssh $(SSH_OPTS) ec2-user@$(ONE)

sftp:
	sftp $(SSH_OPTS) ec2-user@ws0.mk48.io

fmt:
	go fmt mk48/...
