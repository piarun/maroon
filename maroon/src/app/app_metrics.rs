use opentelemetry::{KeyValue, global};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use types::range_key::{KeyOffset, KeyRange};

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
