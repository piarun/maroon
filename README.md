# `maroon`

The Rust playground where we put all the pieces together.

Pieces so far:

* https://github.com/dkorolev/rust-experiments
* https://github.com/akantsevoi/maroon-migrator
* https://github.com/akantsevoi/test-environment
* https://github.com/dimacurrentai/migrator

# Visualisation-demonstration of messages inside the system

https://dkorolev.github.io/maroon/

# Useful run scenarious

## Local test in a single node mode

Run in a single-node mode. When you need to test logic, but can sacrifice durability and don't want to start etcd cluster.
```bash
make run-local PORT=3000 CONSENSUS_NODES=1
```

Runs imitation of gateway with the given key-range
- If you run several gateways - each of them should have their own KEY_RANGE
- NODE_URLS specifies nodes which gateway will try to connect to
```bash
make run-gateway KEY_RANGE=1 NODE_URLS=/ip4/127.0.0.1/tcp/3000
```

# to-do list
- [X] local run of etcd in docker compose
    - [X] add possibility to introduce delays between etcd nodes
- [X] MN. deploy empty application in N exemplars in docker-compose that just writes to the log
- [X] MN. node order + calculating delays
    - [X] MN. establish p2p connection between all the nodes(exchange public keys) + send pings
    - [X] MN. calculate delay for each node
- [X] MN. regularly exchange current vector state to all MNs
- [X] G. Minimal gateway implementation that just publishes transactions
- [X] MN. Request outdated transactions(p2p)
- [X] MN. Fix "epoch" (local)
- [ ] MN. integration with state machine - "puf-puf-magic"
- [ ] epoch coordinator implementation
  - [x] epoch coordinator interface
  - [x] set up etcd
  - [x] write epochs to etcd
  // TODO: update instructions on how to local run it properly. Explain why docerized tests exist. How to run them, etc.
  - [ ] calculate delay for each node to send epoch. Use calculated order and last commited epoch author
- [ ] G/MN. Add API to request key ranges for G
    - [ ] MN. store used ranges on etcd
- [ ] dump data to s3?? (??: what exactly we need to persist? Format? Easy to bootstrap later??)
- [ ] write script that finds leader, pauses the container and then restores it after a new leader elected
- [ ] G. make it working as a server/sidecar/library
- [ ] MN. Bootstratp node from s3
