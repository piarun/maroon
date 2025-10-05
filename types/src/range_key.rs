use derive_more::{Add, AddAssign, Display, Sub};
use serde::{Deserialize, Serialize};

// TODO: KeyRange and KeyOffset shouldn't be u64 since their combination fits into u64
//
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub struct KeyRange(pub u64);

#[derive(
  Serialize, Deserialize, AddAssign, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Add, Display, Sub,
)]
pub struct KeyOffset(pub u64);

/// Unique identifier for a transaction
#[derive(
  Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Add, AddAssign, Sub, Display,
)]
pub struct UniqueU64BlobId(pub u64);

impl From<u64> for UniqueU64BlobId {
  fn from(value: u64) -> Self {
    UniqueU64BlobId(value)
  }
}

/// [a,b] = {x ∈ ℕ | a <= x <= b}
///
/// ```rust
/// use types::range_key::U64BlobIdClosedInterval;
/// use types::range_key::UniqueU64BlobId;
///
/// let interval = U64BlobIdClosedInterval::new(UniqueU64BlobId(0), UniqueU64BlobId(2));
/// assert_eq!(3, interval.ids_count());
///
/// let interval = U64BlobIdClosedInterval::new(UniqueU64BlobId(0), UniqueU64BlobId(0));
/// assert_eq!(1, interval.ids_count());
///
/// ```

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct U64BlobIdClosedInterval {
  left: UniqueU64BlobId,
  right: UniqueU64BlobId,
}

impl U64BlobIdClosedInterval {
  pub fn new<T: Into<UniqueU64BlobId>>(
    left: T,
    right: T,
  ) -> U64BlobIdClosedInterval {
    let left = left.into();
    let right = right.into();
    if right < left {
      panic!("never do it")
    };

    U64BlobIdClosedInterval { left, right }
  }

  pub fn new_from_range_and_offsets(
    range: KeyRange,
    left: KeyOffset,
    right: KeyOffset,
  ) -> U64BlobIdClosedInterval {
    U64BlobIdClosedInterval::new(
      unique_blob_id_from_range_and_offset(range, left),
      unique_blob_id_from_range_and_offset(range, right),
    )
  }

  pub fn ids_count(&self) -> usize {
    let diff = self.right.0 - self.left.0;
    match diff.checked_add(1) {
      Some(count) => count as usize,
      None => panic!("interval too large: would overflow when adding 1"),
    }
  }

  pub fn start(&self) -> UniqueU64BlobId {
    self.left
  }

  pub fn end(&self) -> UniqueU64BlobId {
    self.right
  }

  pub fn iter(&self) -> impl Iterator<Item = UniqueU64BlobId> + '_ {
    let mut current = self.left;
    std::iter::from_fn(move || {
      if current <= self.right {
        let result = current;
        current.0 += 1;
        Some(result)
      } else {
        None
      }
    })
  }
}

///
const SINGLE_BLOB_SIZE: u64 = 1 << 30; // 1_073_741_824
const MAX_BLOCK_INDEX: u64 = (1 << (64 - 30)) - 1; // [0:17_179_869_184)

pub fn full_interval_for_range(range: KeyRange) -> U64BlobIdClosedInterval {
  if range.0 > MAX_BLOCK_INDEX {
    panic!("index can't be more than {}", MAX_BLOCK_INDEX);
  }

  U64BlobIdClosedInterval::new(
    UniqueU64BlobId(range.0 * SINGLE_BLOB_SIZE),
    UniqueU64BlobId(range.0 * SINGLE_BLOB_SIZE + SINGLE_BLOB_SIZE - 1),
  )
}

pub fn range_from_unique_blob_id(global_key: UniqueU64BlobId) -> KeyRange {
  if global_key.0 > MAX_BLOCK_INDEX * SINGLE_BLOB_SIZE {
    panic!("out of range");
  }

  KeyRange(global_key.0 / SINGLE_BLOB_SIZE)
}

/// Converts full id (UniqueU64BlobId) into range and offset.
/// ```
/// use types::range_key::unique_blob_id_from_range_and_offset;
/// use types::range_key::UniqueU64BlobId;
/// use types::range_key::range_offset_from_unique_blob_id;
///
/// let id = UniqueU64BlobId(10);
/// let (range, offset) = range_offset_from_unique_blob_id(id);
/// let id_from = unique_blob_id_from_range_and_offset(range, offset);
/// assert_eq!(id, id_from);
///
/// ```
pub fn range_offset_from_unique_blob_id(global_key: UniqueU64BlobId) -> (KeyRange, KeyOffset) {
  let range = range_from_unique_blob_id(global_key);
  let offset = global_key.0 % SINGLE_BLOB_SIZE;
  (range, KeyOffset(offset))
}

pub fn unique_blob_id_from_range_and_offset(
  range: KeyRange,
  offset: KeyOffset,
) -> UniqueU64BlobId {
  UniqueU64BlobId(range.0 * SINGLE_BLOB_SIZE + offset.0)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_range_offset_from_unique_blob_id() {
    let tests = vec![
      (UniqueU64BlobId(0), KeyRange(0), KeyOffset(0)),
      (UniqueU64BlobId(1), KeyRange(0), KeyOffset(1)),
      (UniqueU64BlobId(1_073_741_824), KeyRange(1), KeyOffset(0)),
    ];

    for (key, ex_range, ex_offset) in tests {
      let (range, offset) = range_offset_from_unique_blob_id(key);
      assert_eq!(range, ex_range, "key: {}", key.0);
      assert_eq!(offset, ex_offset, "key: {}", key.0);
      assert_eq!(unique_blob_id_from_range_and_offset(range, offset), key);
    }
  }

  #[test]
  fn test_transaction_id_operation() {
    let tx1 = UniqueU64BlobId(10);
    let tx2 = UniqueU64BlobId(15);
    assert_eq!(UniqueU64BlobId(5), tx2 - tx1);
  }

  #[test]
  #[should_panic(expected = "interval too large: would overflow when adding 1")]
  fn test_unique_blob_overflow() {
    let interval = U64BlobIdClosedInterval::new(UniqueU64BlobId(0), UniqueU64BlobId(u64::MAX));
    interval.ids_count();
  }

  #[test]
  #[should_panic]
  fn test_unique_blob_creation() {
    _ = U64BlobIdClosedInterval::new(10, 0);
  }
  #[test]
  fn test_correct_sorting() {
    let mut intervals = vec![
      U64BlobIdClosedInterval::new(10, 20),
      U64BlobIdClosedInterval::new(1, 5),
      U64BlobIdClosedInterval::new(11, 14),
      U64BlobIdClosedInterval::new(8, 16),
    ];

    intervals.sort();

    assert_eq!(
      vec![
        U64BlobIdClosedInterval::new(1, 5),
        U64BlobIdClosedInterval::new(8, 16),
        U64BlobIdClosedInterval::new(10, 20),
        U64BlobIdClosedInterval::new(11, 14),
      ],
      intervals
    );
  }

  #[test]
  fn test_interval_iterator() {
    let interval = U64BlobIdClosedInterval::new(5, 8);
    let mut iter = interval.iter();

    assert_eq!(Some(UniqueU64BlobId(5)), iter.next());
    assert_eq!(Some(UniqueU64BlobId(6)), iter.next());
    assert_eq!(Some(UniqueU64BlobId(7)), iter.next());
    assert_eq!(Some(UniqueU64BlobId(8)), iter.next());
    assert_eq!(None, iter.next());

    let empty = U64BlobIdClosedInterval::new(5, 5);
    let mut iter = empty.iter();
    assert_eq!(Some(UniqueU64BlobId(5)), iter.next());
    assert_eq!(None, iter.next());
  }
}
