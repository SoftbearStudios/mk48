# game.mk
#
# SPDX-FileCopyrightText: 2024 Softbear, Inc.
# SPDX-License-Identifier: LGPL-3.0-or-later
#

.PHONY: fmt run run_debug rustup submodules

all: fmt

engine/.git:
	make submodules

fmt:
	cargo fmt --manifest-path client/Cargo.toml
	cargo fmt --manifest-path common/Cargo.toml
	if [ -d "~/macros" ]; then cargo fmt --manifest-path macros/Cargo.toml; fi
	if [ -d "~/scene" ]; then cargo fmt --manifest-path scene/Cargo.toml; fi
	cargo fmt --manifest-path server/Cargo.toml
	if [ -d "~/server_fuzzer" ]; then cargo fmt --manifest-path server_fuzzer/Cargo.toml; fi
	if [ -d "~/sprite_sheet_packer" ]; then cargo fmt --manifest-path sprite_sheet_packer/Cargo.toml; fi

run_debug: engine/.git
	clear
	(cd client; make debug) && (cd server; make run_debug)

run_release: engine/.git
	clear
	(cd client; make release) && (cd server; make run_release)

rustup:
	rustup override set nightly-2024-04-20
	rustup target add wasm32-unknown-unknown

trunk:
	cargo install --locked trunk --version 0.21.7
	(cd client; trunk clean --tools)

tools: rustup trunk

submodules:
	git submodule init
	git submodule update

clean:
	cargo clean --manifest-path engine/Cargo.toml
	cargo clean --manifest-path client/Cargo.toml
	cargo clean --manifest-path common/Cargo.toml
	cargo clean --manifest-path server/Cargo.toml
