use common::range_key::UniqueU64BlobId;
use epoch_coordinator::epoch::Epoch;
use log::debug;

pub trait Linearizer {
  fn new_epoch(
    &mut self,
    epoch: Epoch,
  );
}

pub struct LogLineriazer {
  sequence: Vec<UniqueU64BlobId>,
}

impl LogLineriazer {
  pub fn new() -> LogLineriazer {
    LogLineriazer { sequence: vec![] }
  }
}

impl Linearizer for LogLineriazer {
  fn new_epoch(
    &mut self,
    mut epoch: Epoch,
  ) {
    epoch.increments.sort();
    let new_elements_count: usize = epoch.increments.iter().map(|i| i.ids_count()).sum();
    self.sequence.reserve(new_elements_count);

    for interval in &epoch.increments {
      for i in interval.iter() {
        self.sequence.push(i);
      }
    }

    debug!("new log: {:?}", self.sequence);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use common::logical_time::LogicalTimeAbsoluteMs;
  use common::range_key::{KeyOffset, KeyRange, U64BlobIdClosedInterval, unique_blob_id_from_range_and_offset};
  use libp2p::PeerId;

  #[test]
  fn test_linear() {
    let mut linearizer = LogLineriazer::new();
    let peer_id = PeerId::random();

    linearizer.new_epoch(Epoch::next(
      peer_id,
      vec![
        U64BlobIdClosedInterval::new_from_range_and_offsets(KeyRange(3), KeyOffset(0), KeyOffset(0)),
        U64BlobIdClosedInterval::new_from_range_and_offsets(KeyRange(0), KeyOffset(0), KeyOffset(2)),
        U64BlobIdClosedInterval::new_from_range_and_offsets(KeyRange(1), KeyOffset(0), KeyOffset(3)),
      ],
      None,
      LogicalTimeAbsoluteMs(100),
    ));
    linearizer.new_epoch(Epoch::next(
      peer_id,
      vec![
        U64BlobIdClosedInterval::new_from_range_and_offsets(KeyRange(4), KeyOffset(0), KeyOffset(1)),
        U64BlobIdClosedInterval::new_from_range_and_offsets(KeyRange(0), KeyOffset(3), KeyOffset(5)),
      ],
      None,
      LogicalTimeAbsoluteMs(200),
    ));

    assert_eq!(
      vec![
        unique_blob_id_from_range_and_offset(KeyRange(0), KeyOffset(0)),
        unique_blob_id_from_range_and_offset(KeyRange(0), KeyOffset(1)),
        unique_blob_id_from_range_and_offset(KeyRange(0), KeyOffset(2)),
        unique_blob_id_from_range_and_offset(KeyRange(1), KeyOffset(0)),
        unique_blob_id_from_range_and_offset(KeyRange(1), KeyOffset(1)),
        unique_blob_id_from_range_and_offset(KeyRange(1), KeyOffset(2)),
        unique_blob_id_from_range_and_offset(KeyRange(1), KeyOffset(3)),
        unique_blob_id_from_range_and_offset(KeyRange(3), KeyOffset(0)),
        unique_blob_id_from_range_and_offset(KeyRange(0), KeyOffset(3)),
        unique_blob_id_from_range_and_offset(KeyRange(0), KeyOffset(4)),
        unique_blob_id_from_range_and_offset(KeyRange(0), KeyOffset(5)),
        unique_blob_id_from_range_and_offset(KeyRange(4), KeyOffset(0)),
        unique_blob_id_from_range_and_offset(KeyRange(4), KeyOffset(1)),
      ],
      linearizer.sequence
    );
  }
}
