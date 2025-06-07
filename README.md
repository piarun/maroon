# `maroon`

The Rust playground where we put all the pieces together.

Pieces so far:

* https://github.com/dkorolev/rust-experiments
* https://github.com/akantsevoi/maroon-migrator
* https://github.com/akantsevoi/test-environment
* https://github.com/dimacurrentai/migrator

# Visualisation-demonstration of messages inside the system

https://dkorolev.github.io/maroon/

## Run instructions

```
cargo build
```

```
RUST_BACKTRACE=1 cargo test -- --nocapture
```

Run in a single-node mode. When you don't need other nodes => no consensus, no durability but good for high-level testing
```bash
make run-local PORT=3000 CONSENSUS_NODES=1
```

Runs imitation of gateway with the given key-range
- If you run several gateways - each of them should have their own KEY_RANGE
- NODE_URLS specifies nodes which gateway will try to connect to
```bash
make run-gateway KEY_RANGE=1 NODE_URLS=/ip4/127.0.0.1/tcp/3000
```