# Mk48.io Game

[![Go](https://github.com/SoftbearStudios/mk48/actions/workflows/go.yml/badge.svg)](https://github.com/SoftbearStudios/mk48/actions/workflows/go.yml)

![Logo](/client/static/logo-712.png)

[Mk48.io](https://mk48.io) is an online multiplayer naval combat game, in which you take command of a ship and sail your way to victory. Watch out for torpedoes!

## Developing

### Client

0. Install `NodeJS 14` or higher
1. Enter `/client`
2. `npm install`
3. `make` or `npm run dev`
4. Navigate to http://localhost:3000

### Server

0. Install `go1.16` or higher
1. Enter `/server`
2. `make`
3. Profile with `make pprof` and optionally specify `seconds=<number>` and/or `profile=heap`

## Contributing
See [Contributing](https://github.com/SoftbearStudios/mk48/wiki/Contributing) Wiki page.
