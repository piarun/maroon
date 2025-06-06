.DEFAULT_GOAL := help

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

.PHONY: help fmt toolinstall test build integtest run-local run-gateway

help:
	@echo "Available targets:"
	@awk 'BEGIN {FS = ":"} /^[a-zA-Z_-]+:/ { target = $$1; if ($$0 ~ /#/) { match($$0, /.*#[ \t]*(.*)/, arr); help = arr[1] } else { help = "no help" } printf "  %-15s %s\n", target, help }' $(MAKEFILE_LIST)

toolinstall: # installs tools
	cargo install taplo-cli

build:
	cargo build $(PROFILE_FLAG) $(VERBOSE_RUN)

test: # runs unit tests
	cargo test --workspace --exclude integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- $(NOCAPTURE)

integtest: # runs integration tests
	RUST_LOG=maroon=info,gateway=debug \
		cargo test -p integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)

run-local: # runs maroon node locally on a specified port
	NODE_URLS=/ip4/127.0.0.1/tcp/3000,/ip4/127.0.0.1/tcp/3001,/ip4/127.0.0.1/tcp/3002 \
	SELF_URL=/ip4/127.0.0.1/tcp/${PORT} \
	RUST_LOG=maroon=debug \
	CONSENSUS_NODES=${CONSENSUS_NODES} \
		cargo run -p maroon $(PROFILE_FLAG)

run-gateway: # runs gateway imitation 
	KEY_RANGE=${KEY_RANGE} \
	NODE_URLS=${NODE_URLS} \
	RUST_LOG=gateway=debug \
		cargo run -p gateway $(PROFILE_FLAG)

shutdown-etcd: # shutdown and clean up local etcd cluster
	docker compose -f deploy/etcd/docker-compose.yaml down --remove-orphans
	docker network rm etcd

start-etcd: # run etcd for local development
	docker network create etcd
	docker compose -f deploy/etcd/docker-compose.yaml up -d


fmt: # formatter
	cargo fmt --all
	taplo format