PROFILE ?= debug
VERBOSE ?= ""
PORT ?= 3000
NODE_URLS ?= /ip4/127.0.0.1/tcp/3000,/ip4/127.0.0.1/tcp/3001,/ip4/127.0.0.1/tcp/3002
KEY_RANGE ?= 0
CONSENSUS_NODES ?= 2

ifeq ($(PROFILE),release)
    PROFILE_FLAG := --release
endif

ifeq ($(VERBOSE),true)
    VERBOSE_RUN := --verbose
	NOCAPTURE := --nocapture
endif

.PHONY: fmt toolinstall test build integtest run-local run-gateway

toolinstall:
	cargo install taplo-cli

build:
	cargo build $(PROFILE_FLAG) $(VERBOSE_RUN)

test:
	cargo test --workspace --exclude integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- $(NOCAPTURE)

integtest:
	RUST_LOG=maroon=info,gateway=debug \
		cargo test -p integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)

run-local:
	NODE_URLS=/ip4/127.0.0.1/tcp/3000,/ip4/127.0.0.1/tcp/3001,/ip4/127.0.0.1/tcp/3002 \
	SELF_URL=/ip4/127.0.0.1/tcp/${PORT} \
	RUST_LOG=maroon=debug \
	CONSENSUS_NODES=${CONSENSUS_NODES} \
		cargo run -p maroon $(PROFILE_FLAG)

run-gateway:
	KEY_RANGE=${KEY_RANGE} \
	NODE_URLS=${NODE_URLS} \
	RUST_LOG=gateway=debug \
		cargo run -p gateway $(PROFILE_FLAG)

fmt:
	cargo fmt --all
	taplo format