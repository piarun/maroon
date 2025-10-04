use generated::maroon_assembler::Value;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use types::range_key::UniqueU64BlobId;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Transaction {
  pub id: UniqueU64BlobId,
  pub status: TxStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Meta {
  pub id: UniqueU64BlobId,
  pub status: TxStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBlueprint {
  pub global_id: UniqueU64BlobId,
  //   pub fiber_type: FiberType,

  // function key to provide an information which function should be executed, ex: `add` or `sub`...
  pub function_key: String,
  // input parameters for the function
  pub init_values: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum TxStatus {
  Created,
  Pending,
  // Confirmed,
  Finished,
  // Rejected,
}
