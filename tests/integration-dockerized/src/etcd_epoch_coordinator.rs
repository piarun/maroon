#![allow(unused_imports)]

use common::duplex_channel::{Endpoint, create_a_b_duplex_pair};
use common::range_key::U64BlobIdClosedInterval;
use etcd_client::{Client, Compare, CompareOp, DeleteOptions, Error, Txn, TxnOp, WatchOptions, WatchResponse};
use libp2p::PeerId;
use maroon::epoch_coordinator::etcd::MAROON_PREFIX;
use maroon::epoch_coordinator::{
  epoch::Epoch,
  etcd::EtcdEpochCoordinator,
  interface::{EpochRequest, EpochUpdates},
};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn etcd_epoch_coordinator() {
  _ = env_logger::try_init();

  let node_urls =
    vec!["http://localhost:2379".to_string(), "http://localhost:2380".to_string(), "http://localhost:2381".to_string()];

  // cleanup etcd before run
  {
    let mut client = Client::connect(node_urls.clone(), None).await.unwrap();
    _ = client.delete(MAROON_PREFIX, Some(DeleteOptions::new().with_prefix())).await.expect("expect deletion");
  }

  let (Endpoint::<EpochRequest, EpochUpdates> { mut receiver, sender }, b2a) =
    create_a_b_duplex_pair::<EpochRequest, EpochUpdates>();

  let peer_id_1 = PeerId::random();
  let coordinator = EtcdEpochCoordinator::new(&node_urls, b2a);

  let task = tokio::spawn(async move {
    while let Some(v) = receiver.recv().await {
      return Some(v);
    }
    None
  });
  coordinator.start_on_background();

  let epoch = Epoch::next(peer_id_1, vec![U64BlobIdClosedInterval::new(0, 13)], None);
  _ = sender.send(EpochRequest { epoch: epoch.clone() });

  let updates = task.await.unwrap().unwrap();

  assert_eq!(EpochUpdates::New(epoch), updates);
}
