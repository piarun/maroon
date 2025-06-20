use super::epoch::Epoch;
use crate::interface::{B2AEndpoint, EpochRequest, EpochUpdates};
use derive_more::Display;
use etcd_client::{Client, Compare, CompareOp, Error, Txn, TxnOp, WatchOptions, WatchResponse};
use log::{error, info, warn};
use opentelemetry::{KeyValue, global, metrics::Counter};
use serde::{Deserialize, Serialize};
use std::{sync::OnceLock, time::Duration};

fn counter_requests() -> &'static Counter<u64> {
  static COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
  COUNTER.get_or_init(|| global::meter("etcd_epoch_coordinator").u64_counter("etcd_requests").build())
}

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
  interface: B2AEndpoint,
}

impl EtcdEpochCoordinator {
  pub fn new(etcd_endpoints: &Vec<String>, interface: B2AEndpoint) -> EtcdEpochCoordinator {
    EtcdEpochCoordinator { etcd_endpoints: etcd_endpoints.clone(), interface }
  }

  /// starts infinite loop. After this all the communications with corrdinator only through `EpochCoordinatorInterface`
  pub async fn start(self) -> Result<(), Error> {
    info!("start epoch coordinator");

    let mut interface = self.interface;

    let mut client = Client::connect(self.etcd_endpoints, None).await?;
    let mut last_rev: Option<i64> = None;

    let mut watcher_creation_timeout = Duration::from_millis(50);

    loop {
      let options = {
        let mut w = WatchOptions::new().with_prefix();
        if let Some(r) = last_rev {
          w = w.with_start_revision(r);
        }
        w
      };
      let watch_result = client.watch(MAROON_LATEST, Some(options)).await;

      let (watcher, mut watch_stream) = match watch_result {
        Ok(res) => {
          watcher_creation_timeout = Duration::from_millis(50);
          res
        }
        Err(e) => {
          error!("create watcher err: {:?}", e);
          counter_requests().add(1, &[KeyValue::new("error", true)]);
          if watcher_creation_timeout <= Duration::from_secs(5) {
            watcher_creation_timeout = watcher_creation_timeout * 2;
          }
          tokio::time::sleep(watcher_creation_timeout).await;
          continue;
        }
      };

      // let (watcher, mut watch_stream) = watch_result.unwrap();
      // Keep watcher alive within the task scope
      let _watcher = watcher;

      loop {
        tokio::select! {
          Some(payload) = interface.receiver.recv() => {
            handle_commit_new_epoch(&mut client, payload).await;
          },
          watch_result = watch_stream.message() => match watch_result{
            Ok(Some(message)) => {
              if let Some(h) = message.header() {
                last_rev = Some(h.revision());
              }
              handle_watch_message(&mut interface, message);
            }
            Ok(None) => {
              // Server cleanly closed the watch (EOF)
              warn!("etcd watch stream closed by server; reconnecting...");
              break; // breaks inner loop - reconnect
            }
            Err(e) => {
              error!("etcd watch error: {e}; reconnecting...");
              break; // breaks inner loop - reconnect
            },
          },
        }
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

fn handle_watch_message(interface: &mut B2AEndpoint, message: WatchResponse) {
  for event in message.events() {
    if let Some(kv) = event.kv() {
      if let Ok(epoch_obj) = serde_json::from_slice::<EpochObject>(kv.value()) {
        info!("etcd watch got {} epoch", epoch_obj.epoch.sequence_number);
        interface.send(EpochUpdates::New(epoch_obj.epoch));
      }
    }
  }
}

// TODO: return here an error and write a dockerized-test to set/update latest
// because right now the logic is not covered reliably
async fn handle_commit_new_epoch(client: &mut Client, epoch_request: EpochRequest) {
  let seq_number = epoch_request.epoch.sequence_number;
  let new_epoch = EpochObject { epoch: epoch_request.epoch };
  let resp = client
    .txn(
      Txn::new()
        .when(vec![Compare::version(format!("{}/{}", MAROON_HISTORY, seq_number), CompareOp::Equal, 0)])
        .and_then(vec![
          TxnOp::put(MAROON_LATEST, serde_json::to_vec(&new_epoch).unwrap(), None),
          TxnOp::put(format!("{}/{}", MAROON_HISTORY, seq_number), serde_json::to_vec(&new_epoch).unwrap(), None),
        ]),
    )
    .await;

  match resp {
    Ok(result) => {
      info!("commit {} epoch success: {:?}", seq_number, result.succeeded());
      counter_requests().add(1, &[KeyValue::new("success", result.succeeded())]);
    }
    Err(e) => {
      error!("commit {} epoch err: {:?}", seq_number, e);
      counter_requests().add(1, &[KeyValue::new("error", true)]);
    }
  }
}
