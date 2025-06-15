.DEFAULT_GOAL := help

PROFILE ?= debug
VERBOSE ?= ""
PORT ?= 3000
NODE_URLS ?= /ip4/127.0.0.1/tcp/3000,/ip4/127.0.0.1/tcp/3001,/ip4/127.0.0.1/tcp/3002
ETCD_URLS ?= http://localhost:2379,http://localhost:2380,http://localhost:2381

KEY_RANGE ?= 0
CONSENSUS_NODES ?= 2

ifeq ($(PROFILE),release)
    PROFILE_FLAG := --release
endif

ifeq ($(VERBOSE),true)
    VERBOSE_RUN := --verbose
	NOCAPTURE := --nocapture
endif

.PHONY: help fmt toolinstall test build integtest integtest-dockerized integtest-all run-local run-gateway

help:
	@echo "Synopsis:"
	@(grep -E '^[a-zA-Z0-9_-]+:.*#.*' $(MAKEFILE_LIST) \
	  | sed 's/^\([^:]*\):.*#\(.*\)/\1\t\2/' \
	  | awk -F'\t' '{ printf "  make \033[1;36m%23s\033[0m :%s\n", $$1, $$2 }')

toolinstall: # installs tools
	cargo install taplo-cli

build:
	cargo build --all-targets $(PROFILE_FLAG) $(VERBOSE_RUN)

test: # runs unit tests
	cargo test --workspace --exclude integration --exclude epoch_coordinator $(PROFILE_FLAG) $(VERBOSE_RUN) -- $(NOCAPTURE)

integtest: # runs integration tests (excluding dockerized tests)
	RUST_LOG=maroon=info,gateway=debug \
		cargo test -p integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)

integtest-dockerized: # runs dockerized integration tests that require docker services (etcd, etc.)
	RUST_LOG=maroon=debug,gateway=debug \
		cargo test -p epoch_coordinator $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)

integtest-all: # runs all integration tests including dockerized ones
	RUST_LOG=maroon=info,gateway=debug \
		cargo test -p integration -p integration-dockerized $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)

run-local: # runs maroon node locally on a specified port
	NODE_URLS=${NODE_URLS} \
	ETCD_URLS=${ETCD_URLS} \
	SELF_URL=/ip4/127.0.0.1/tcp/${PORT} \
	RUST_LOG=maroon=debug,epoch_coordinator=debug \
	CONSENSUS_NODES=${CONSENSUS_NODES} \
		cargo run -p maroon $(PROFILE_FLAG)

run-gateway: # runs gateway imitation
	KEY_RANGE=${KEY_RANGE} \
	NODE_URLS=${NODE_URLS} \
	RUST_LOG=gateway=debug \
		cargo run -p gateway $(PROFILE_FLAG)

shutdown-test-etcd: # shutdown and clean up local etcd cluster
	docker compose -f epoch_coordinator/docker/etcd/docker-compose.yaml down --remove-orphans
	docker network rm etcd
	docker volume rm etcd_etcd-data

start-test-etcd: # run etcd for local development
	docker network create etcd
	docker compose -f epoch_coordinator/docker/etcd/docker-compose.yaml up -d

fmt: # formatter
	cargo fmt --all
	taplo format
