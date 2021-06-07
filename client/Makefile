.PHONY=dev build install

all: dev

node_modules:
	npm install

dev: node_modules src/data/entities.json
	npm run dev

../server/world/entities.json: ../server/world/entities-raw.json
	( cd ../server/world; node preprocess.mjs; )

src/data/entities.json: ../server/world/entities.json
	cp ../server/world/entities.json src/data/entities.json

build: src/*/*.js src/*/*.svelte src/* node_modules src/data/entities.json
	npm run build

install: build
	aws s3 cp --profile mk48 --cache-control no-cache --recursive build/ s3://mk48-prod-static/
