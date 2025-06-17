use common::log_events::{CommandBody, LogEvent};
use rand::Rng;
use serde_json::to_string;

fn main() {
    let mut rng = rand::thread_rng();
    let gateway_id = rng.gen::<u64>();
    let client_ids = [rng.gen::<u64>(), rng.gen::<u64>(), rng.gen::<u64>()];
    let event_ids = (0..60).map(|_| rng.gen::<u64>()).collect::<Vec<_>>();

    println!("{}", to_string(&LogEvent::GatewayUp { gid: gateway_id }).unwrap());

    for &cid in &client_ids {
        println!("{}", to_string(&LogEvent::ClientConnected { cid }).unwrap());
    }

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

    for &cid in &client_ids {
        for i in 0..20 {
            let event_id = event_ids[i];
            let message = messages[i % messages.len()];
            println!(
                "{}",
                to_string(&LogEvent::ClientSentCommand {
                    cid,
                    eid: event_id,
                    gid: gateway_id,
                    body: CommandBody::TextMessageCommand(message.to_string()),
                })
                .unwrap()
            );
        }
    }

    for &cid in &client_ids {
        println!("{}", to_string(&LogEvent::ClientDisconnected { cid }).unwrap());
    }

    println!("{}", to_string(&LogEvent::GatewayDown { gid: gateway_id }).unwrap());
} 