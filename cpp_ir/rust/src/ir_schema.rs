#![allow(unused_imports)]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRVarRegular {
  pub line: u32,
  pub name: String,
  pub r#type: String,
  pub init: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRVarFunctionArg {
  pub line: u32,
  pub name: String,
  pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRVarEnumCaseCapture {
  pub name: String,
  pub key: String,
  pub src: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MaroonIRVar {
  MaroonIRVarRegular(MaroonIRVarRegular),
  MaroonIRVarFunctionArg(MaroonIRVarFunctionArg),
  MaroonIRVarEnumCaseCapture(MaroonIRVarEnumCaseCapture),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRStmt {
  pub line: u32,
  pub stmt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRIf {
  pub line: u32,
  pub cond: String,
  pub yes: Box<MaroonIRStmtOrBlock>,
  pub no: Box<MaroonIRStmtOrBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRMatchEnumStmtArm {
  pub line: u32,
  pub key: Option<String>,
  pub capture: Option<String>,
  pub code: MaroonIRBlock,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRMatchEnumStmt {
  pub line: u32,
  pub var: String,
  pub arms: Vec<MaroonIRMatchEnumStmtArm>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRBlockPlaceholder {
  pub line: u32,
  pub _idx: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MaroonIRStmtOrBlock {
  MaroonIRStmt(MaroonIRStmt),
  MaroonIRIf(MaroonIRIf),
  MaroonIRBlock(MaroonIRBlock),
  MaroonIRMatchEnumStmt(MaroonIRMatchEnumStmt),
  MaroonIRBlockPlaceholder(MaroonIRBlockPlaceholder),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRBlock {
  pub line: u32,
  pub vars: Vec<MaroonIRVar>,
  pub code: Vec<MaroonIRStmtOrBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRFunction {
  pub line: u32,
  pub ret: Option<String>,
  pub args: Vec<String>,
  pub body: MaroonIRBlock,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRFiber {
  pub line: u32,
  pub functions: BTreeMap<String, MaroonIRFunction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRTypeDefStructField {
  pub name: String,
  pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRTypeDefStruct {
  pub fields: Vec<MaroonIRTypeDefStructField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRTypeDefEnumCase {
  pub key: String,
  pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRTypeDefEnum {
  pub cases: Vec<MaroonIRTypeDefEnumCase>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRTypeDefOptional {
  pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MaroonIRTypeDef {
  MaroonIRTypeDefStruct(MaroonIRTypeDefStruct),
  MaroonIRTypeDefEnum(MaroonIRTypeDefEnum),
  MaroonIRTypeDefOptional(MaroonIRTypeDefOptional),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRType {
  pub line: u32,
  pub def: Box<MaroonIRTypeDef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRNamespace {
  pub line: u32,
  pub fibers: BTreeMap<String, MaroonIRFiber>,
  pub types: BTreeMap<String, MaroonIRType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonTestCaseRunFiber {
  pub line: u32,
  pub maroon: String,
  pub fiber: String,
  pub golden_output: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonTestCaseFiberShouldThrow {
  pub line: u32,
  pub maroon: String,
  pub fiber: String,
  pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MaroonTestCase {
  MaroonTestCaseRunFiber(MaroonTestCaseRunFiber),
  MaroonTestCaseFiberShouldThrow(MaroonTestCaseFiberShouldThrow),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaroonIRScenarios {
  // The source `.mrn` file.
  pub src: String,
  pub maroon: BTreeMap<String, MaroonIRNamespace>,
  pub tests: Vec<MaroonTestCase>,
}
