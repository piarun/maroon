#![allow(unused_imports)]

use common::duplex_channel::{Endpoint, create_a_b_duplex_pair};
use common::range_key::U64BlobIdClosedInterval;
use epoch_coordinator::etcd::{self, MAROON_PREFIX};
use epoch_coordinator::{
  epoch::Epoch,
  etcd::EtcdEpochCoordinator,
  interface::{EpochRequest, EpochUpdates},
};
use etcd_client::{Client, Compare, CompareOp, DeleteOptions, Error, Txn, TxnOp, WatchOptions, WatchResponse};
use libp2p::PeerId;
use std::time::Duration;
use testcontainers::{
  GenericImage, ImageExt,
  core::{IntoContainerPort, WaitFor},
  runners::AsyncRunner,
};

/// Testing that when we send tx to etcd we'll get it back through "watch" api
/// not sure it's useful test, but anyway
#[tokio::test(flavor = "multi_thread")]
async fn etcd_epoch_coordinator() {
  _ = env_logger::try_init();

  let container = GenericImage::new("quay.io/coreos/etcd", "v3.5.0")
    .with_entrypoint("etcd")
    .with_exposed_port(2379.tcp())
    .with_exposed_port(2380.tcp())
    .with_wait_for(WaitFor::message_on_stderr("ready to serve client requests"))
    .with_network("bridge")
    .with_cmd(vec![
      "--name",
      "single-node",
      "--data-dir",
      "/etcd-data",
      "--listen-client-urls",
      "http://0.0.0.0:2379",
      "--advertise-client-urls",
      "http://0.0.0.0:2379",
      "--listen-peer-urls",
      "http://0.0.0.0:2380",
      "--initial-advertise-peer-urls",
      "http://0.0.0.0:2380",
      "--initial-cluster",
      "single-node=http://0.0.0.0:2380",
      "--initial-cluster-state",
      "new",
    ])
    .start()
    .await
    .expect("failed to get etcd");

  let port = container.get_host_port_ipv4(2379).await.unwrap();
  let etcd_url = format!("http://127.0.0.1:{}", port);

  let node_urls = vec![etcd_url];

  let (Endpoint::<EpochRequest, EpochUpdates> { mut receiver, sender }, b2a) =
    create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();

  let peer_id_1 = PeerId::random();
  let coordinator = EtcdEpochCoordinator::new(&node_urls, b2a);

  coordinator.start_on_background();

  let epoch = Epoch::next(peer_id_1, vec![U64BlobIdClosedInterval::new(0, 13)], None);
  let epoch2 = Epoch::next(peer_id_1, vec![U64BlobIdClosedInterval::new(14, 16)], Some(&epoch));

  _ = sender.send(EpochRequest { epoch: epoch.clone() });
  let updates = receiver.recv().await.expect("can it be None?");
  assert_eq!(EpochUpdates::New(epoch), updates);

  _ = sender.send(EpochRequest { epoch: epoch2.clone() });
  let updates = receiver.recv().await.expect("can it be None?");
  assert_eq!(EpochUpdates::New(epoch2), updates);
}
