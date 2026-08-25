# map-generator — the practical verbs.
# On Windows without make, run the same scripts directly:
#   bash scripts/demo.sh start|stop   and   bash scripts/make-maps.sh

PORT ?= 8090

.PHONY: build test demo stop maps artifacts clean

build:
	cargo build --release

test:
	cargo test

# The standing workbench demo, launched DETACHED (session-spawned
# background tasks have been killed out from under it; see project
# memory demo-launch-detached).
demo: build
	bash scripts/demo.sh start

stop:
	bash scripts/demo.sh stop

# A bunch of maps for the Bible: renders the canonical set through the
# running demo's API into out/maps/ (SVG, scalable). Needs `make demo`.
maps:
	PORT=$(PORT) bash scripts/make-maps.sh

# Serverless content-addressed plates: no server needed. Filenames are
# the cache key (query hash + world pin); manifest.json maps names to
# artifacts. Output: out/artifacts/.
artifacts: build
	./target/release/map-cli plates

clean:
	cargo clean
	rm -rf out
