PROFILE ?= debug
VERBOSE ?= ""

ifeq ($(PROFILE),release)
    PROFILE_FLAG := --release
endif

ifeq ($(VERBOSE),true)
    VERBOSE_RUN := --verbose
	NOCAPTURE := --nocapture
endif

.PHONY: fmt toolinstall test build

toolinstall:
	cargo install taplo-cli

build:
	cargo build $(PROFILE_FLAG) $(VERBOSE_RUN)

test:
	cargo test --workspace --exclude integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- $(NOCAPTURE)

integtest:
	RUST_LOG=maroon=info,gateway=debug \
		cargo test -p integration $(PROFILE_FLAG) $(VERBOSE_RUN) -- --test-threads 1 $(NOCAPTURE)


fmt:
	cargo fmt --all
	taplo format