SOURCE := $(wildcard src/*.blend)

all: $(patsubst src/%.blend,rendered/%.png,$(wildcard src/*.blend))

rendered/%.png: src/%.blend src/common/*.blend
	mkdir -p rendered
	docker run --mount type=bind,src=$(shell pwd),dst=/usr/src/models nytimes/blender:2.93-cpu-ubuntu18.04 blender -b /usr/src/models/$(subst png,blend,$(subst rendered/,src/,$@)) -o /usr/src/models/$@ -f 1
	cp $@0001.png $@
	rm -rf $@0001.png