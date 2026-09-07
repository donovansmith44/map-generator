# map-generator — the practical verbs.
# On Windows without make, run the same scripts directly:
#   bash scripts/demo.sh start|stop   and   bash scripts/make-maps.sh

PORT ?= 8090

.PHONY: build test demo stop maps artifacts clean contract-gates ci

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

# The toolchain-only gates: no server, no browser. Safe anywhere.
contract-gates:
	cd contracts/runner && cabal test
	cd contracts/runner && cabal run contract-runner -- check ../map-api
	cd contracts/runner && cabal run contract-runner -- check ../atlas-edge
	cd contracts/runner && cabal run contract-runner -- vocab ../map-api
	cd contracts/runner && cabal run contract-runner -- vocab ../atlas-edge
	cargo test --workspace
	bash scripts/tests/semver-gate.test.sh

# Everything, including the gates that need the live servers (8090 ours,
# 8080 the atlas) and the browser. Never binds a port itself.
ci: contract-gates
	cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api
	cd contracts/runner && cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge
	bash scripts/contract-semver-gate.sh $${BASE_REF:-origin/master}
	node crates/map-viewer/tests/golden.js --check
