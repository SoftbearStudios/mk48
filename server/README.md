# Mk48.io Server

The server is a Rust application. If compiled in `release` mode, it will embed the (previously compiled) client within itself.
By contrast, when compiling in `debug` mode, the client is served out of its public directory.

## Instructions

0. Install nightly Rust ([here](https://rustup.rs/), then `rustup default nightly-2021-10-28`)
1. `make`
2. Navigate to `localhost:8000`

## Note

You may use any version of Rust that works, but we use some nightly features and `nightly-2021-10-28` is known to work,
whereas some newer versions produce internal compiler errors.