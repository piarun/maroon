.DEFAULT_GOAL := help

PROFILE ?= debug
VERBOSE ?= ""
PORT ?= 3000
NODE_URLS ?= /ip4/127.0.0.1/tcp/3000,/ip4/127.0.0.1/tcp/3001,/ip4/127.0.0.1/tcp/3002
ETCD_URLS ?= http://localhost:2379,http://localhost:2380,http://localhost:2381
GATEWAY_PORT ?= 5000

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

build-images: # builds docker images for maroon/gateway/etc..
	docker build -f maroon/docker/Dockerfile -t maroon:local .
	docker build -f gateway/docker/Dockerfile -t gateway:local .

test: # runs unit tests
	cargo test --workspace --exclude integration --exclude epoch_coordinator $(PROFILE_FLAG) $(VERBOSE_RUN) -- $(NOCAPTURE)

integtest: # runs integration tests (excluding dockerized tests)
	RUST_LOG=maroon=info,gateway=debug \
		cargo test -p integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)

integtest-dockerized: # runs dockerized integration tests that require docker services (etcd, etc.)
	RUST_LOG=maroon=debug,gateway=debug \
		cargo test -p epoch_coordinator $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)
		cargo test -p state_log $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)


integtest-all: # runs all integration tests including dockerized ones
	RUST_LOG=maroon=info,gateway=debug \
		cargo test -p integration -p integration-dockerized $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)

run-local: # runs maroon node locally on a specified port
	OTEL_EXPORTER_OTLP_GRPC_ENDPOINT=http://localhost:4317 \
	OTEL_RESOURCE_ATTRIBUTES=service.name=maroon \
	OTEL_METRIC_EXPORT_INTERVAL=10000 \
	NODE_URLS=${NODE_URLS} \
	ETCD_URLS=${ETCD_URLS} \
	SELF_URL=/ip4/127.0.0.1/tcp/${PORT} \
	REDIS_URL=redis://127.0.0.1:6379 \
	RUST_LOG=debug \
	CONSENSUS_NODES=${CONSENSUS_NODES} \
		cargo run -p maroon $(PROFILE_FLAG)

run-gateway: # runs gateway imitation
	KEY_RANGE=${KEY_RANGE} \
	NODE_URLS=${NODE_URLS} \
	REDIS_URL=redis://127.0.0.1:6379 \
	PORT=${GATEWAY_PORT} \
	RUST_LOG=gateway=info \
		cargo run -p gateway $(PROFILE_FLAG)

shutdown-test-etcd: # shutdown and clean up local etcd cluster
	docker compose -f epoch_coordinator/docker/etcd/docker-compose.yaml down -v --remove-orphans
	docker network rm etcd

start-test-etcd: # run etcd for local development
	docker network create etcd
	docker compose -f epoch_coordinator/docker/etcd/docker-compose.yaml up -d

.PHONY: start-metrics-stack
start-metrics-stack: # starts OTLP collector, prometheus, grafana
	docker compose -f metrics/docker-compose.yaml up -d

.PHONY: shutdown-metrics-stack
shutdown-metrics-stack: # shuts down OTLP collector, prometheus, grafana
	docker compose -f metrics/docker-compose.yaml down -v

.PHONY: start-redis
start-redis: # starts Redis in docker compose for state log
	docker compose -f state_log/docker/redis/docker-compose.yaml up -d

.PHONY: shutdown-redis
shutdown-redis: # shuts down Redis docker compose
	docker compose -f state_log/docker/redis/docker-compose.yaml down -v

.PHONY: start-maroon
start-maroon: # starts maroon cluster by using maroon:local
	docker compose -f maroon/docker/docker-compose.yaml up -d

.PHONY: shutdown-maroon
shutdown-maroon: # shuts down maroon docker compose
	docker compose -f maroon/docker/docker-compose.yaml down -v

.PHONY: start-gateway
start-gateway: # starts gateway cluster by using gateway:local
	docker compose -f gateway/docker/docker-compose.yaml up -d

.PHONY: shutdown-gateway
shutdown-gateway: # shuts down gateway docker compose
	docker compose -f gateway/docker/docker-compose.yaml down -v

.PHONY: maroon-logs
maroon-logs:
	docker compose -f maroon/docker/docker-compose.yaml logs

.PHONY: start-compose
start-compose: start-test-etcd start-metrics-stack start-redis

.PHONY: shutdown-compose
shutdown-compose: shutdown-metrics-stack shutdown-test-etcd shutdown-redis

fmt: # formatter
	cargo fmt --all
	taplo format
