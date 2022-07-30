use bevy::prelude::*;
use bevy_renet::renet::{ClientAuthentication, RenetClient, RenetConnectionConfig};
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use crate::{ClientChannel, ClientHostMessage, ServerChannel, PROTOCOL_ID};
pub struct PunchthroughClientPlugin {
    pub local_socket: SocketAddr,
    pub punchthrough_server: SocketAddr,
}

pub struct AttemptJoinEvent {
    pub lobby: String,
}

pub struct PunchthroughClientRes {
    pub client: RenetClient,
    /// Contains a tuple of the address received from the server swap response, the number of times its been tried, and the last time it was tried
    pub target_addr: Option<(SocketAddr, u16)>,
    pub local_socket: SocketAddr,
    pub punchthrough_server: SocketAddr,
}

impl Plugin for PunchthroughClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AttemptJoinEvent>();
        app.insert_resource(PunchthroughClientRes {
            client: new_renet_client(),
            target_addr: None,
            local_socket: self.local_socket,
            punchthrough_server: self.punchthrough_server,
        });
        app.add_system(punchthrough_system);
    }
}

/// This is the main system of the punchthrough client. It will process server messages and send any responses back up to the server as required
/// When it receives a swap comand it will insert the info into the PunchThroughClientRes.target field. It will then attempt a handshake at preset intervals
pub fn punchthrough_system(
    mut client_res: ResMut<PunchthroughClientRes>,
) {


    while let Some(message) = client_res
        .client
        .receive_message(ClientChannel::Command.id())
    {
        let server_message: ClientHostMessage = bincode::deserialize(&message).unwrap();
        match server_message {
            ClientHostMessage::JoinLobbyResponse { err } => {
                if let Some(error) = err {
                    match error {
                        crate::ClientError::LobbyNotFound { lobby } => {
                            warn!("Lobby not found: {}", lobby)
                        }
                        crate::ClientError::InternalServerError => {
                            warn!("Received ISE in Join Response")
                        }
                    }
                }
            }
            ClientHostMessage::AttemptHandshakeCommand { socket } => {
                //Theoretically you should only have to send one packet to punchthrough
                match send_pt_packet(client_res.local_socket, socket){
                    Ok(()) => {
                        info!("Successfully sent Punchthrough Packet")
                    },
                    Err(e) => {
                        error!("Could not send punchthrough packet to socket {} because {e:#?}", socket.ip());
                    }
                };
            }
            _ => {}
        }
    }
}

pub fn client_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ClientChannel::channels_config(),
        receive_channels_config: ServerChannel::channels_config(),
        ..Default::default()
    }
}

fn new_renet_client() -> RenetClient {
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let connection_config = client_connection_config();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: None,
    };

    RenetClient::new(
        current_time,
        socket,
        client_id,
        connection_config,
        authentication,
    )
    .unwrap()
}

fn send_pt_packet(local_socket: SocketAddr, target_socket: SocketAddr) -> Result<(), String> {
    let local_udp_socket = UdpSocket::bind(local_socket);

    match local_udp_socket {
        Ok(bound_socket) => {
            match bound_socket.connect(target_socket) {
                Ok(_) => {
                    match bound_socket.send("BevyPunchthrough Packet".as_bytes()){
                        Ok(_something) => {
                            info!("Sent Handshake Packet");
                        },
                        Err(e) => {
                            warn!("Could not send Handshake Packet because {e:#?}");
                        }
                    }
                }
                Err(send_error) => {return Err(send_error.to_string());}
            }

            Ok(())
        }
        Err(e) => {
            Err(e.to_string())
        }
    }
}
