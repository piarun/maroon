#![allow(unused_imports)]

use std::{collections::HashMap, num::NonZeroUsize, thread::sleep, time::Duration};

use common::{
  duplex_channel::Endpoint,
  gm_request_response::Request,
  invoker_handler::InvokerInterface,
  meta_exchange::Response,
  range_key::{KeyOffset, KeyRange, UniqueU64BlobId},
  transaction::{Transaction, TxStatus},
};
use gateway::core::Gateway;
use maroon::{
  app::{App, CurrentOffsets, Params, Request as AppRequest, Response as AppResponse},
  stack,
};
use tokio::sync::oneshot;

#[tokio::test(flavor = "multi_thread")]
async fn request_missed_txs() {
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

  let mut gw = Gateway::new(vec!["/ip4/127.0.0.1/tcp/3000".to_string()]).unwrap();

  // run nodes and gateway

  let _s0 = stack0.start();
  let _s1 = stack1.start();
  let _s2 = stack2.start();

  gw.start_in_background().await;

  // wait until they are connected
  tokio::time::sleep(Duration::from_secs(1)).await;

  // send requests from gateway
  _ = gw.send_request(Request::NewTransaction(Transaction { id: UniqueU64BlobId(1), status: TxStatus::Created })).await;
  _ = gw.send_request(Request::NewTransaction(Transaction { id: UniqueU64BlobId(0), status: TxStatus::Created })).await;

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

    tokio::time::sleep(Duration::from_secs(1)).await;

    if node0_correct && node1_correct && node2_correct {
      break;
    }
  }

  assert!(node0_correct);
  assert!(node1_correct);
  assert!(node2_correct);
}
