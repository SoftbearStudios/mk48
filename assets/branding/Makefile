CHROMIUM := $(shell which chromium)
COMMA := ,

all: logo-270x196.png logo-512x340.jpg logo-512x384.jpg logo-512x512.jpg logo-1280x550.jpg logo-1280x720.jpg logo-1400x861.jpg

logo-%.png: logo.html
	$(CHROMIUM) --headless --screenshot='$@' --window-size=$(subst logo-,,$(subst .png,,$(subst x,$(COMMA),$@))) logo.html

%.jpg: %.png
	convert $(subst jpg,png,$@) $@
