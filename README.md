# Mk48.io Game

[![Go](https://github.com/SoftbearStudios/mk48/actions/workflows/go.yml/badge.svg)](https://github.com/SoftbearStudios/mk48/actions/workflows/go.yml)
<a href='https://discord.gg/YMheuFQWTX'>
  <img src='https://img.shields.io/badge/Mk48.io-%23announcements-blue.svg' alt='Mk48.io Discord' />
</a>
<a href='https://discord.gg/UQmcwM9NGr'>
  <img src='https://img.shields.io/badge/Discord%20Gophers-%23mk48io-blue.svg' alt='Discord Gophers' />
</a>

![Logo](/client/static/logo-712.png)

[Mk48.io](https://mk48.io) is an online multiplayer naval combat game, in which you take command of a ship and sail your way to victory. Watch out for torpedoes!

- [Ship Suggestions](https://github.com/SoftbearStudios/mk48/wiki/Ship-Suggestions-&-Plans)

## Developing

### Client

0. Install `NodeJS 14` or higher
1. Enter `/client`
2. `npm install`
3. `make` or `npm run dev`
4. Navigate to http://localhost:3000

### Server

0. Install `go1.16` or higher
1. Enter `/server_main`
2. `make`
3. Profile with `make pprof` and optionally specify `seconds=<number>` and/or `profile=heap`

### Docker

Docker infrastructure is also available, that runs the client and server.

0. Install `docker` and `docker-compose`
1. `docker-compose build`
2. `docker-compose up`
3. Navigate to http://localhost:3000

## Contributing
See [Contributing](https://github.com/SoftbearStudios/mk48/wiki/Contributing) Wiki page.
