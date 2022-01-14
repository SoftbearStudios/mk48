# Mk48.io Server

The server is a Rust application. If compiled in `release` mode, it will embed the (previously compiled) client within itself.
By contrast, when compiling in `debug` mode, the client is served out of its public directory.

## Instructions

0. Install nightly Rust according to the top-level README
1. `make`
2. Navigate to `localhost:8000`