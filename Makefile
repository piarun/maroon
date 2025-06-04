PROFILE ?= debug
VERBOSE ?= ""

ifeq ($(PROFILE),release)
    PROFILE_FLAG := --release
endif

ifeq ($(VERBOSE),true)
    VERBOSE_RUN := --verbose
	NOCAPTURE := --nocapture
endif

.PHONY: fmt toolinstall test

toolinstall:
	cargo install taplo-cli

build:
	cargo build $(PROFILE_FLAG) $(VERBOSE_RUN)

test:
	cargo test --workspace $(PROFILE_FLAG) $(VERBOSE_RUN) -- $(NOCAPTURE)

fmt:
	cargo fmt --all
	taplo format