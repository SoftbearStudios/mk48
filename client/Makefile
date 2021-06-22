.PHONY=dev build install

all: dev

node_modules:
	npm install

dev: node_modules src/data/entities.json static/server.wasm
	npm run dev

../server/world/entities.json: ../data/entities-raw.json ../data/preprocess.mjs
	( cd ../data; node preprocess.mjs; )

src/data/entities.json: ../server/world/entities.json
	cp ../server/world/entities.json src/data/entities.json

static/server.wasm: ../server_wasm/*.go ../server/*.go ../server/*/*.go ../server/*/*/*.go ../server/world/entities.json
	( cd ../server_wasm; GOOS=js GOARCH=wasm go build -o static/server.wasm; )

build: src/*/*.js src/*/*.svelte src/* node_modules src/data/entities.json static/server.wasm
	npm run build

install: build
	aws s3 cp --profile mk48 --cache-control no-cache --recursive build/ s3://mk48-prod-static/
