.PHONY=dev build install

all: dev

node_modules:
	npm install

dev:
	npm run dev

build: src/*/*.js src/*/*.svelte src/*
	npm run build

install: build
	aws s3 cp --profile mk48 --cache-control no-cache --recursive build/ s3://mk48-prod-static/
