use opentelemetry::{global, metrics::Counter};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

static LATEST_EPOCH: AtomicU64 = AtomicU64::new(0);

pub fn register_gauges() {
  static INIT: OnceLock<()> = OnceLock::new();
  INIT.get_or_init(|| {
    let meter = global::meter("maroon_app");

    let latest_epoch = meter
      .u64_observable_gauge("maroon_latest_epoch")
      .with_description("Shows latest epoch this node knows about")
      .with_callback(|observer| {
        let v = LATEST_EPOCH.load(Ordering::Relaxed);
        observer.observe(v, &[]);
      })
      .build();

    // Leak the registration so it's never dropped during process lifetime.
    std::mem::forget(latest_epoch);
  });
}

pub fn set_latest_epoch_seq_number(v: u64) {
  LATEST_EPOCH.store(v, Ordering::Relaxed);
}

pub fn know_txs() -> &'static Counter<u64> {
  static COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
  COUNTER.get_or_init(|| {
    global::meter("maroon_app")
      .u64_counter("maroon_tx_knows")
      .with_description("How many transactions this node knows about")
      .build()
  })
}

// how many transactions on this node are in finished state
pub fn finished_txs() -> &'static Counter<u64> {
  static COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
  COUNTER.get_or_init(|| {
    global::meter("maroon_app")
      .u64_counter("maroon_tx_finished")
      .with_description("How many transactions finished by this node")
      .build()
  })
}
