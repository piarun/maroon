use schema::log_events::{CommandBody, LogEvent, LogEventBody};
use schema::{Cid, Eid, Gid};
use serde_json::to_string;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_time_micros() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64
}

fn main() {
  let gateway_id = Gid::new_random();
  let client_ids = [Cid::new_random(), Cid::new_random(), Cid::new_random()];
  let event_ids = (0..60).map(|_| Eid::new_random()).collect::<Vec<_>>();

  let mut current_time = current_time_micros();

  // The gateway starts up first.
  println!(
    "{}",
    to_string(&LogEvent { timestamp_micros: current_time, body: LogEventBody::GatewayUp { gid: gateway_id } }).unwrap()
  );

  // We wait one second before clients start connecting.
  current_time += 1_000_000;

  // Clients connect with 200ms intervals between each connection.
  for &cid in &client_ids {
    println!(
      "{}",
      to_string(&LogEvent { timestamp_micros: current_time, body: LogEventBody::ClientConnected { cid } }).unwrap()
    );
    current_time += 200_000; // 200ms between client connections.
  }

  // We wait 500ms before messages start flowing.
  current_time += 500_000;

  let messages = [
    "Hello gateway!",
    "How are you?",
    "What's the weather like?",
    "Can you help me?",
    "I need assistance",
    "Is everything ok?",
    "Just checking in",
    "Any updates?",
    "Status report please",
    "Are you there?",
  ];

  // Messages are sent with 100ms intervals between each message.
  for &cid in &client_ids {
    for i in 0..20 {
      let event_id = event_ids[i];
      let message = messages[i % messages.len()];
      println!(
        "{}",
        to_string(&LogEvent {
          timestamp_micros: current_time,
          body: LogEventBody::ClientSentCommand {
            cid,
            eid: event_id,
            gid: gateway_id,
            body: CommandBody::TextMessageCommand(message.to_string()),
          },
        })
        .unwrap()
      );
      current_time += 100_000; // 100ms between messages.
    }
  }

  // We wait one second before clients start disconnecting.
  current_time += 1_000_000;

  // Clients disconnect with 300ms intervals between each disconnection.
  for &cid in &client_ids {
    println!(
      "{}",
      to_string(&LogEvent { timestamp_micros: current_time, body: LogEventBody::ClientDisconnected { cid } }).unwrap()
    );
    current_time += 300_000; // 300ms between client disconnections.
  }

  // We wait two seconds before the gateway shuts down.
  current_time += 2_000_000;

  println!(
    "{}",
    to_string(&LogEvent { timestamp_micros: current_time, body: LogEventBody::GatewayDown { gid: gateway_id } })
      .unwrap()
  );
}
