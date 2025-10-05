#![allow(unused_imports)]

use std::{collections::HashMap, num::NonZeroUsize, thread::sleep, time::Duration};

use common::{
  duplex_channel::Endpoint,
  invoker_handler::InvokerInterface,
  range_key::{KeyOffset, KeyRange, UniqueU64BlobId},
};
use gateway::core::Gateway;
use generated::maroon_assembler::Value;
use maroon::{
  app::{App, CurrentOffsets, Params, Request as AppRequest, Response as AppResponse},
  stack,
};
use protocol::gm_request_response::Request;
use protocol::meta_exchange::Response;
use protocol::transaction::{FiberType, Meta, TaskBlueprint, Transaction, TxStatus};
use tokio::sync::oneshot;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
  _ = env_logger::try_init();

  let params = Params::default().set_advertise_period(Duration::from_millis(500));

  // create nodes and gateway

  let (stack0, remote_control_0) = stack::MaroonStack::new(
    vec!["/ip4/127.0.0.1/tcp/3001".to_string(), "/ip4/127.0.0.1/tcp/3002".to_string()],
    vec![],
    "/ip4/0.0.0.0/tcp/3000".to_string(),
    params.clone(),
  )
  .unwrap();
  let (stack1, remote_control_1) = stack::MaroonStack::new(
    vec!["/dns4/localhost/tcp/3000".to_string(), "/dns4/localhost/tcp/3002".to_string()],
    vec![],
    "/ip4/0.0.0.0/tcp/3001".to_string(),
    params.clone(),
  )
  .unwrap();
  let (stack2, remote_control_2) = stack::MaroonStack::new(
    vec!["/ip4/127.0.0.1/tcp/3000".to_string(), "/ip4/127.0.0.1/tcp/3001".to_string()],
    vec![],
    "/ip4/0.0.0.0/tcp/3002".to_string(),
    params,
  )
  .unwrap();

  let mut gw = Gateway::new(
    KeyRange(0),
    vec![
      "/ip4/127.0.0.1/tcp/3000".to_string(),
      "/ip4/127.0.0.1/tcp/3001".to_string(),
      "/ip4/127.0.0.1/tcp/3002".to_string(),
    ],
  )
  .unwrap();

  // run nodes and gateway

  let _s0 = stack0.start();
  let _s1 = stack1.start();
  let _s2 = stack2.start();

  gw.start_in_background().await;

  // wait until they are connected
  tokio::time::sleep(Duration::from_secs(1)).await;

  gw.send_request(test_add_blueprint(2, 4), None).await;
  gw.send_request(test_add_blueprint(2, 4), None).await;

  // check results
  let (mut node0_correct, mut node1_correct, mut node2_correct) = (false, false, false);

  let get_state_and_compare =
    async |interface: &InvokerInterface<AppRequest, AppResponse>, offsets: &CurrentOffsets| -> bool {
      let app_state_response = interface.request(AppRequest::GetState).await;
      println!("got app: {app_state_response:?}");

      let AppResponse::State(app_state) = app_state_response;
      app_state == *offsets
    };
  let desired_state = CurrentOffsets {
    self_offsets: HashMap::from([(KeyRange(0), KeyOffset(1))]),
    consensus_offset: HashMap::from([(KeyRange(0), KeyOffset(1))]),
  };

  for _ in 0..3 {
    node0_correct = get_state_and_compare(&remote_control_0.state_invoker, &desired_state).await;
    node1_correct = get_state_and_compare(&remote_control_1.state_invoker, &desired_state).await;
    node2_correct = get_state_and_compare(&remote_control_2.state_invoker, &desired_state).await;
    if node0_correct && node1_correct && node2_correct {
      break;
    }
    println!("TICK");
    tokio::time::sleep(Duration::from_secs(1)).await;
  }

  assert!(node0_correct);
  assert!(node1_correct);
  assert!(node2_correct);
}

#[cfg(test)]
fn test_add_blueprint(
  a: u64,
  b: u64,
) -> TaskBlueprint {
  TaskBlueprint {
    fiber_type: FiberType::new("global"),
    function_key: "add".to_string(),
    init_values: vec![Value::U64(a), Value::U64(b)],
  }
}
