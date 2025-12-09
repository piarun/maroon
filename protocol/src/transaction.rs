use generated::maroon_assembler::Value;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use types::range_key::UniqueU64BlobId;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Transaction {
  pub meta: Meta,
  pub blueprint: TaskBlueprint,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Meta {
  pub id: UniqueU64BlobId,
  pub status: TxStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TxUpdate {
  pub meta: Meta,
  pub result: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBlueprint {
  /// registered in ir_spec public queue
  pub queue_name: String,

  /// message with the same type as queue accepts it
  pub param: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum TxStatus {
  Created,
  Pending,
  // Confirmed,
  Finished,
  /// if smth is wrong with the request. Ex: wrong queue, incorrect message type, etc.
  Rejected(String),
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub struct FiberType(pub String);
impl std::fmt::Display for FiberType {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl FiberType {
  pub fn new(id: impl Into<String>) -> Self {
    Self(id.into())
  }
}

impl std::borrow::Borrow<str> for FiberType {
  fn borrow(&self) -> &str {
    self.0.as_str()
  }
}
