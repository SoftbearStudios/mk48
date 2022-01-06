# Mk48.io Game

[![Build](https://github.com/SoftbearStudios/mk48/actions/workflows/build.yml/badge.svg)](https://github.com/SoftbearStudios/mk48/actions/workflows/build.yml)
<a href='https://discord.gg/YMheuFQWTX'>
  <img src='https://img.shields.io/badge/Mk48.io-%23announcements-blue.svg' alt='Mk48.io Discord' />
</a>

![Logo](/js/public/logo-712.png)

[Mk48.io](https://mk48.io) is an online multiplayer naval combat game, in which you take command of a ship and sail your way to victory. Watch out for torpedoes!

- [Ship Suggestions](https://github.com/SoftbearStudios/mk48/discussions/132)

## Developing

### Client

0. Install NodeJS 14 or higher ([here](https://nodejs.org/en/download/)) and nightly Rust ([here](https://rustup.rs/), then `rustup default nightly`)
1. Enter `/js`
2. `npm install`
3. `make`
4. Deploy the server to host the client

### Server

0. Install nightly Rust ([here](https://rustup.rs/), then `rustup default nightly-2021-10-28`)
1. Enter `/server`
2. `make`
3. Navigate to `localhost:8000`

## Contributing
See [Contributing](https://github.com/SoftbearStudios/mk48/wiki/Contributing) Wiki page.
