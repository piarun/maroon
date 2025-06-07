// Integration tests that require Docker services (etcd, etc.)
// These tests assume external services are running and available

/// 1. Start the etcd cluster: `make start-etcd`
/// 2. Run dockerized tests: `make integtest-dockerized`
/// 3. Stop the etcd cluster: `make shutdown-etcd`
pub mod etcd_epoch_coordinator;
