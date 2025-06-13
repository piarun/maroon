use common::range_key::U64BlobIdClosedInterval;
use derive_more::Display;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Display, Serialize, Deserialize, PartialEq, Eq)]
#[display("Epoch {{ sn: {:?} increments: {:?}, hash: 0x{:X} }}", sequence_number, increments, hash.iter().fold(0u128, |acc, &x| (acc << 8) | x as u128))]
pub struct Epoch {
  /// order number of an epoch
  /// starts with 0
  pub sequence_number: u64,

  pub increments: Vec<U64BlobIdClosedInterval>,

  pub creator: PeerId,

  pub creation_time: Duration,

  hash: [u8; 32],
}

impl Epoch {
  pub fn next(creator: PeerId, increments: Vec<U64BlobIdClosedInterval>, prev_epoch: Option<&Epoch>) -> Epoch {
    let mut hasher = Sha256::new();

    let mut sequence_number = 0;
    // Include previous hash if it exists
    if let Some(prev) = prev_epoch {
      hasher.update(prev.hash);
      sequence_number = prev.sequence_number + 1;
    }

    // Include current epoch data
    for interval in &increments {
      hasher.update(interval.start().0.to_le_bytes());
      hasher.update(interval.end().0.to_le_bytes());
    }

    hasher.update(creator.to_bytes());

    let hash = hasher.finalize().into();

    Epoch {
      creator,
      sequence_number,
      increments,
      hash,
      creation_time: SystemTime::now().duration_since(UNIX_EPOCH).expect("it's way after unix epoch start"),
    }
  }
}
