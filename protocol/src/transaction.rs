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
  pub fiber_type: FiberType,

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
