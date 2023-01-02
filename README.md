# Mk48.io Game

[![Build](https://github.com/SoftbearStudios/mk48/actions/workflows/build.yml/badge.svg)](https://github.com/SoftbearStudios/mk48/actions/workflows/build.yml)
<a href='https://discord.gg/YMheuFQWTX'>
  <img src='https://img.shields.io/badge/Mk48.io-%23announcements-blue.svg' alt='Mk48.io Discord' />
</a>

![Logo](/client/logo-712.png)

[Mk48.io](https://mk48.io) is an online multiplayer naval combat game, in which you take command of a ship and sail your way to victory. Watch out for torpedoes!

- [Ship Suggestions](https://github.com/SoftbearStudios/mk48/discussions/132)

## Building

### Tools

0. Install `rustup` ([see instructions here](https://rustup.rs/))
1. Install Rust Nightly (`rustup override set nightly-2022-08-14`)
2. Add WebAssembly target (`rustup target add wasm32-unknown-unknown`)
3. Install `trunk` (`cargo install --locked trunk`, install `gcc` first if it complains about missing `cc`)
4. Optionally, install the `make` command

You may use any version of Rust that works, but only `nightly-2022-08-14` is known
to be compatible.

### Client

0. Enter `/client`
1. `make` or, equivalently, `trunk build --release`
2. Deploy the server to host the client

### Server

0. Enter `/server`
1. `make` to build and run a test server
2. Navigate to `localhost:8081` (or whatever port is printed in the console)

### HTTPS

If you build the server in `--release` mode, it will force HTTPS using a self-signed certificate.

Optionally, specify `--certificate-path` and `--private-key-path` to use a trusted CA certificate (e.g. acquired via [Let's Encrypt](https://letsencrypt.org/)). The server will periodically check for and load renewed certificates.

## Developing

If you follow the *Building* steps, you have a fully functioning game (could be used to host a private server). If your goal
is to modify the game, you may want to read more :)

### Entity data

Entities (ships, weapons, aircraft, collectibles, obstacles, decoys, etc.) are defined at the bottom of
`common/src/entity/_type.rs`.

### Entity textures

Each entity type must be accompanied by a texture of the same name in the spritesheet, which comes with the
repository. If entity textures need to be changed, see instructions in the `sprite_sheet_packer` directory.

### Engine

Both client and server rely on our custom game engine (which is present in the `engine` directory).

#### Admin interface (optional)
One notable feature of the engine is an (optional) admin interface. To build it:

0. Install NodeJS 14 or higher ([here](https://nodejs.org/en/download/))
1. Enter `/engine/js`
2. `make`
3. Deploy the server to host the admin interface
4. Go to `localhost:8080/admin`
5. Paste the contents of `engine/game_server/src/auth.txt`, generated randomly by a build script, into the alert dialog

### Macros

Many macros are utilized by the codebase. Mk48-specific macros can be found in the `macros` directory,
and game engine macros can be found in the `engine/engine_macros` directory. A few notable macros are:
- Mk48 entity loader (generates `EntityData` for all entity types, used by both client and server)
- Game engine audio loader (generates `Audio` enum for client, with a variant per sound)
- Game engine settings (generates Javascript bindings for settings structs)
- Game engine renderer layer (for composable rendering layers)

## Contributing
See [Contributing](https://github.com/SoftbearStudios/mk48/wiki/Contributing) Wiki page.
