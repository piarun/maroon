use super::epoch::Epoch;
use crate::interface::{EpochRequest, EpochUpdates, Interface};
use derive_more::Display;
use etcd_client::{Client, Compare, CompareOp, Error, Txn, TxnOp, WatchOptions, WatchResponse};
use log::{error, info, warn};
use opentelemetry::{
  KeyValue, global,
  metrics::{Counter, Histogram},
};
use serde::{Deserialize, Serialize};
use std::{sync::OnceLock, time::Duration};
use tokio::sync::{mpsc::UnboundedSender, watch::Receiver};
use tokio::time::Instant;

fn epoch_coordinator_requests_to_etcd_counter() -> &'static Counter<u64> {
  static COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
  COUNTER
    .get_or_init(|| global::meter("etcd_epoch_coordinator").u64_counter("epoch_coordinator_requests_to_etcd").build())
}

fn histogram_etcd_latency() -> &'static Histogram<u64> {
  static COUNTER: OnceLock<Histogram<u64>> = OnceLock::new();

  // in milliseconds
  COUNTER.get_or_init(|| {
    global::meter("etcd_epoch_coordinator")
      .u64_histogram("epoch_coordinator_commit_to_etcd_ms")
      .with_boundaries(vec![
        5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 250.0, 500.0, 750.0, 1000.0, 2500.0, 5000.0, 7500.0, 10000.0,
      ])
      .build()
  })
}

fn epoch_coordinator_maroon_commited_transactions_counter() -> &'static Counter<u64> {
  static COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
  COUNTER.get_or_init(|| {
    global::meter("etcd_epoch_coordinator").u64_counter("epoch_coordinator_maroon_commited_transactions").build()
  })
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
  new_epoch_receiver: Receiver<Option<EpochRequest>>,
  epoch_updates_sender: UnboundedSender<EpochUpdates>,
}

impl EtcdEpochCoordinator {
  pub fn new(
    etcd_endpoints: &Vec<String>,
    interface: Interface,
  ) -> EtcdEpochCoordinator {
    EtcdEpochCoordinator {
      etcd_endpoints: etcd_endpoints.clone(),
      new_epoch_receiver: interface.receiver,
      epoch_updates_sender: interface.sender,
    }
  }

  /// starts infinite loop. After this all the communications with corrdinator only through `EpochCoordinatorInterface`
  pub async fn start(self) -> Result<(), Error> {
    info!("start epoch coordinator");

    let mut client = Client::connect(self.etcd_endpoints, None).await?;
    let mut last_rev: Option<i64> = None;
    let mut last_committed_sn: Option<u64> = None;

    let mut sender = self.epoch_updates_sender;
    let mut receiver = self.new_epoch_receiver;

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
          epoch_coordinator_requests_to_etcd_counter().add(1, &[KeyValue::new("success", "error")]);
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
          _ = receiver.changed() => {
              let next = receiver.borrow_and_update().clone();
              if let Some(payload) = next {
                  let sn = payload.epoch.sequence_number;
                  if Some(sn) == last_committed_sn {
                    continue;
                  }

                  let success = handle_commit_new_epoch(&mut client, payload).await;
                  if success {
                      last_committed_sn = Some(sn);
                  }
              }
          },
          watch_result = watch_stream.message() => match watch_result{
            Ok(Some(message)) => {
              if let Some(h) = message.header() {
                last_rev = Some(h.revision());
              }
              handle_watch_message(&mut sender, message);
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

fn handle_watch_message(
  sender: &mut UnboundedSender<EpochUpdates>,
  message: WatchResponse,
) {
  for event in message.events() {
    if let Some(kv) = event.kv() {
      if let Ok(epoch_obj) = serde_json::from_slice::<EpochObject>(kv.value()) {
        info!("etcd watch got {} epoch", epoch_obj.epoch.sequence_number);
        if let Err(e) = sender.send(EpochUpdates::New(epoch_obj.epoch)) {
          error!("failed to send epoch update: {}", e);
        }
      }
    }
  }
}

// TODO: return here an error and write a dockerized-test to set/update latest
// because right now the logic is not covered reliably
// return true if the epoch was successfully committed
async fn handle_commit_new_epoch(
  client: &mut Client,
  epoch_request: EpochRequest,
) -> bool {
  let start = Instant::now();

  let seq_number = epoch_request.epoch.sequence_number;
  let count_new_txs = epoch_request.epoch.increments.iter().map(|r| r.ids_count() as u64).sum();
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

  let mut success = false;

  let labels = match resp {
    Ok(result) => {
      info!("commit {} epoch success: {:?}", seq_number, result.succeeded());
      epoch_coordinator_maroon_commited_transactions_counter().add(count_new_txs, &[]);
      success = result.succeeded();
      vec![KeyValue::new("success", result.succeeded())]
    }
    Err(e) => {
      error!("commit {} epoch err: {:?}", seq_number, e);
      vec![KeyValue::new("success", "error")]
    }
  };

  epoch_coordinator_requests_to_etcd_counter().add(1, &labels);
  histogram_etcd_latency().record(start.elapsed().as_millis() as u64, &labels);

  success
}
