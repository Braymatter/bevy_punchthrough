use std::{time::Duration, net::{SocketAddr}};

use bevy_renet::renet::{NETCODE_KEY_BYTES, ChannelConfig, UnreliableChannelConfig, ReliableChannelConfig};
use serde::{Deserialize, Serialize};

pub mod server;
pub mod client;


pub const PRIVATE_KEY: &[u8; NETCODE_KEY_BYTES] = b"an example very very secret key."; // 32-bytes
pub const PROTOCOL_ID: u64 = 7;

pub enum ClientChannel {
    Input,
    Command,
}

pub enum ServerChannel {
    ServerMessages,
    NetworkFrame,
}

///Informs the server that the application at address would like to allow punch through connections
///Server will store this info and make it available for SwapRequests until it receives the disconnect event
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ClientHostMessage{
    HostNewLobby,
    NewLobbyResponse {lobby_id: String},
    RequestSwap {lobby_id: String},
    JoinLobbyResponse {err: Option<ClientError>},
    AttemptHandshakeCommand {socket: SocketAddr}
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientError{
    LobbyNotFound {lobby: String},
    InternalServerError,
}

impl ClientChannel {
    pub fn id(&self) -> u8 {
        match self {
            Self::Input => 0,
            Self::Command => 1,
        }
    }

    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ReliableChannelConfig {
                channel_id: Self::Input.id(),
                message_resend_time: Duration::ZERO,
                ..Default::default()
            }
            .into(),
            ReliableChannelConfig {
                channel_id: Self::Command.id(),
                message_resend_time: Duration::ZERO,
                ..Default::default()
            }
            .into(),
        ]
    }
}

impl ServerChannel {
    pub fn id(&self) -> u8 {
        match self {
            Self::NetworkFrame => 0,
            Self::ServerMessages => 1,
        }
    }

    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            UnreliableChannelConfig {
                channel_id: Self::NetworkFrame.id(),
                ..Default::default()
            }
            .into(),
            ReliableChannelConfig {
                channel_id: Self::ServerMessages.id(),
                message_resend_time: Duration::from_millis(200),
                ..Default::default()
            }
            .into(),
        ]
    }
}
