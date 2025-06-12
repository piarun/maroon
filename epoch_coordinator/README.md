# what

This component has some logic but important function of it is "wrapping" etcd.

Since it's a "wrapper" around etcd it's hard to test it without etcd like we do for tests/integration (inside one process). That's why we have here `./docker/etcd` folder that contains all the necessary files to run etcd cluster in docker compose.

## run tests
1. `make start-test-etcd`
2. `make integtest-dockerized`
3. `make shutdown-test-etcd`
