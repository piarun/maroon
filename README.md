# `maroon`

The Rust playground where we put all the pieces together.

Pieces so far:

* https://github.com/dkorolev/rust-experiments
* https://github.com/akantsevoi/maroon-migrator
* https://github.com/akantsevoi/test-environment
* https://github.com/dimacurrentai/migrator

# Visualisation-demonstration of messages inside the system

[https://dimacurrentai.github.io/maroon/util/visualize_log.html](https://dimacurrentai.github.io/maroon/util/visualize_log.html).

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

## +- realistic run scenarious

1. Run etcd:
- `make start-test-etcd`

2. Run in a separate terminal sessions(if you need different ports, provide correct NODE_URLS as well):
- `make run-local PORT=3000`
- `make run-local PORT=3001`
- `make run-local PORT=3002`

3. Now if you open another terminal session you can see updates in etcd:
- `etcdctl --endpoints=http://localhost:2379 get --prefix /maroon/history`
- [check out more](./epoch_coordinator/docker/etcd/Readme.md)

you should see published epochs but with empty increments

4. In order to start publishing transactions - use [gateway](./docs/gateway.md).

Right now it's very dumb implementation that can only publish empty transactions from a given [key-range](./docs/keys-range.md). To run:
```sh
make run-gateway KEY_RANGE=2 NODE_URLS=/ip4/127.0.0.1/tcp/3000
```

NODE_URLS should contain at least one valid node url, in that case transaction will reach out all nodes in cluster eventually


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
  - [x] calculate delay for each node to send epoch. Use calculated order and last commited epoch author
- [ ] G/MN. Add API to request key ranges for G
    - [ ] MN. store used ranges on etcd
- [ ] dump data to s3?? (??: what exactly we need to persist? Format? Easy to bootstrap later??)
- [ ] write script that finds leader, pauses the container and then restores it after a new leader elected
- [ ] G. make it working as a server/sidecar/library
- [ ] MN. Bootstratp node from s3
