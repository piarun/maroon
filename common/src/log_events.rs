use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandBody {
    TextMessageCommand(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEvent {
    ClientConnected { cid: u64 },
    ClientDisconnected { cid: u64 },
    GatewayUp { gid: u64 },
    GatewayDown { gid: u64 },
    ClientSentCommand {
        cid: u64,
        eid: u64,
        gid: u64,
        body: CommandBody,
    },
} 