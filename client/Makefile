GAME_ID = Mk48

all: release

include ../makefiles/client.mk

manifest:
	cargo run --manifest-path ../engine/manifest/Cargo.toml -- \
		--name mk48.io \
		--domain mk48.io \
		--description "Mk48.io is a multiplayer ship combat game. It is free to play online in a web browser with no install required. Start with a small warship and defeat rival players or collect floating barrels to earn upgrades to increasingly larger battleships, submarines, and aircraft carriers. Avoid torpedos and incoming shells. Team and chat with other players." \
		--keywords ship,submarine,torpedo \
		--background-color "#00487d" \
		--theme-color "#00487d" \
		--icons 256x256.png,mask256x256.png,512x512.png,512x340.png,1280x720.png,1920x1080.png \
		--screenshots ss1920x1080.png

