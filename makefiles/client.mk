# client.mk
#
# SPDX-FileCopyrightText: 2024 Softbear, Inc.
# SPDX-License-Identifier: LGPL-3.0-or-later
#
# Do NOT define 'all' or 'renderer' target herein because these may vary.

TRUNK_OPTS = --minify --no-sri --skip-version-check --filehash false

.PHONY: all clean debug features licenses manifest release release_std release_std_log remove_phrases renderer touch_engine translations


clean: remove_phrases
	cargo clean

debug:
	trunk build $(TRUNK_OPTS) index.dev.html
	mv dist/index.dev.html dist/index.html

features:
	clear
	trunk build index.dev.html --all-features

licenses:
	cargo run --manifest-path ../engine/licensing/Cargo.toml -- --binary ../client --binary ../server --format md > src/ui/translations/licenses.md

release:
	trunk build --release $(TRUNK_OPTS)

release_std:
	CARGO_UNSTABLE_BUILD_STD=std,panic_abort CARGO_UNSTABLE_BUILD_STD_FEATURES=panic_immediate_abort trunk build --release $(TRUNK_OPTS)

release_std_log:
	CARGO_UNSTABLE_BUILD_STD=std,panic_abort CARGO_UNSTABLE_BUILD_STD_FEATURES=panic_immediate_abort trunk build --release --features log $(TRUNK_OPTS)

remove_phrases:
	rm -f ../client/phrases.txt ../engine/client/phrases.txt

touch_engine:
	touch ../engine/client/src/lib.rs

translations: remove_phrases touch_engine release
ifdef GAME_ID
	sort ../client/phrases.txt | uniq | cargo run --manifest-path ../engine/uploader/Cargo.toml -- $(GAME_ID)
endif
	sort ../engine/client/phrases.txt | uniq | cargo run --manifest-path ../engine/uploader/Cargo.toml -- Engine
