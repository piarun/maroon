use super::epoch::Epoch;
use crate::epoch_coordinator::interface::{EpochCoordinatorInterface, EpochRequest, EpochUpdates};
use derive_more::Display;
use etcd_client::{Client, Compare, CompareOp, Error, Txn, TxnOp, WatchOptions, WatchResponse};
use log::debug;
use serde::{Deserialize, Serialize};

pub const MAROON_PREFIX: &str = "/maroon";
const MAROON_LATEST: &str = "/maroon/latest";
const MAROON_HISTORY: &str = "/maroon/history";

/// implementation uses etcd as a backend for EpochCoordinator
///
/// Etcd keys structure:
///
/// /maroon
///   /history
///     /0
///       epoch_obj_0
///     /1
///       epoch_obj_1
///     ...
///     /100500
///       epoch_obj_100500
///   /latest
///     epoch_obj_100500
///
///
/// TODO: introduce some compaction or GC for older objects in history and keep the size relatively small (200-300 objects in history)

pub struct EtcdEpochCoordinator {
  etcd_endpoints: Vec<String>,
  interface: EpochCoordinatorInterface,
}

impl EtcdEpochCoordinator {
  pub fn new(etcd_endpoints: &Vec<String>, interface: EpochCoordinatorInterface) -> EtcdEpochCoordinator {
    EtcdEpochCoordinator { etcd_endpoints: etcd_endpoints.clone(), interface }
  }

  /// starts infinite loop. After this all the communications with corrdinator only through `EpochCoordinatorInterface`
  pub async fn start(self) -> Result<(), Error> {
    let mut client = Client::connect(self.etcd_endpoints, None).await?;

    let (watcher, mut watch_stream) = client.watch(MAROON_LATEST, Some(WatchOptions::new().with_prefix())).await?;
    // Keep watcher alive within the task scope
    let _watcher = watcher;

    let mut interface = self.interface;

    loop {
      tokio::select! {
        Some(payload) = interface.receiver.recv() => {
          handle_commit_new_epoch(&mut client, payload).await;
        },
        Ok(Some(message)) = watch_stream.message() => {
          handle_watch_message(&mut interface, message);
        },
      }
    }
  }

  /// same as `start` but spawns a background tokio thread
  pub fn start_on_background(self) {
    tokio::spawn(async move { self.start().await.expect("no error") });
  }
}

#[derive(Deserialize, Serialize, Debug, Display)]
struct EpochObject {
  epoch: Epoch,
}

fn handle_watch_message(interface: &mut EpochCoordinatorInterface, message: WatchResponse) {
  debug!("etcd watch got: {} messages", message.events().len());
  for event in message.events() {
    if let Some(kv) = event.kv() {
      if let Ok(epoch_obj) = serde_json::from_slice::<EpochObject>(kv.value()) {
        interface.send(EpochUpdates::New(epoch_obj.epoch));
      }
    }
  }
}

async fn handle_commit_new_epoch(client: &mut Client, epoch_request: EpochRequest) {
  debug!("Got message to send to etcd: {:?}", &epoch_request);
  let new_epoch = EpochObject { epoch: epoch_request.epoch };

  let resp = client
    .txn(Txn::new().when(vec![Compare::version(format!("{}/{}", MAROON_HISTORY, 0), CompareOp::Equal, 0)]).and_then(
      vec![
        TxnOp::put(MAROON_LATEST, serde_json::to_vec(&new_epoch).unwrap(), None),
        TxnOp::put(format!("{}/{}", MAROON_HISTORY, 0), serde_json::to_vec(&new_epoch).unwrap(), None),
      ],
    ))
    .await
    .unwrap();

  debug!("NewEpochResponse succeed: {}. Details: {:?}", resp.succeeded(), resp);
}
