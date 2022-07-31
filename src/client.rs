use bevy::prelude::*;
use bevy_renet::renet::{ClientAuthentication, RenetClient, RenetConnectionConfig};
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use crate::{ClientChannel, ClientHostMessage, ServerChannel, PROTOCOL_ID, ClientError};
pub struct PunchthroughClientPlugin {
    pub local_socket: SocketAddr,
    pub punchthrough_server: SocketAddr,
}

#[derive(Debug)]
pub enum RequestSwap {
    JoinLobby {lobby: String},
    HostLobby
}

/// This is the egress point of the plugin. Client apps should listen for this event
#[derive(Debug)]
pub enum PunchthroughEvent {
    Success {target_sock: SocketAddr, local_sock: SocketAddr},
    HostSuccess {lobby: String},
    Failed {reason: String}
}

pub struct PunchthroughClientRes {
    /// Contains a tuple of the address received from the server swap response, the number of times its been tried, and the last time it was tried
    pub target_addr: Option<(SocketAddr, u16)>,
    pub local_socket: SocketAddr,
    pub punchthrough_server: SocketAddr,
}

impl Plugin for PunchthroughClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RequestSwap>();
        app.add_event::<PunchthroughEvent>();
        app.insert_resource(new_renet_client());
        app.insert_resource(PunchthroughClientRes {
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
    mut client_connect_request: EventReader<RequestSwap>,
    mut punchthrough_events: EventWriter<PunchthroughEvent>,
    mut client: ResMut<RenetClient>,
    mut client_res: ResMut<PunchthroughClientRes>,
) {
    while let Some(message) = client
        .receive_message(ClientChannel::Command.id())
    {
        let server_message: ClientHostMessage = bincode::deserialize(&message).unwrap();
        info!("Client received message from server: {server_message:#?}");
        match server_message {
            ClientHostMessage::JoinLobbyResponse { err } => {
                match err {
                        Some(ClientError::LobbyNotFound { lobby }) => {
                            warn!("Lobby not found: {}", lobby)
                        }
                        Some(ClientError::InternalServerError) => {
                            warn!("Received ISE in Join Response")
                        },
                        None => {info!("Successfully Swapped")}
                };
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
            ClientHostMessage::NewLobbyResponse{lobby_id} => {
                punchthrough_events.send(PunchthroughEvent::HostSuccess { lobby: lobby_id });
            },
            _ => {}
        }
    }

    //Probably should just do 1 
    for connect_request in client_connect_request.iter() {
        match connect_request {
            RequestSwap::JoinLobby { lobby } => {
                let swap_req_msg = bincode::serialize(&ClientHostMessage::RequestSwap { lobby_id: lobby.clone() }).expect("Could not serialize request to swap");
                client.send_message(ClientChannel::Command.id(), swap_req_msg);
                info!("Sent Join Lobby Request {connect_request:#?}");

            },
            RequestSwap::HostLobby => {
                let host_req_msg = bincode::serialize(&ClientHostMessage::HostNewLobby).expect("Could not serialize HostNewLobby Enum to bytes");
                client.send_message(ClientChannel::Command.id(), host_req_msg);
                info!("Sent Host Lobby Request");

            }
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

    let socket = UdpSocket::bind("127.0.0.1:5001").unwrap();

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

    let client = RenetClient::new(
        current_time,
        socket,
        client_id,
        connection_config,
        authentication,
    )
    .unwrap();

    println!("Constructed new RenetClient with server addr {server_addr} and client addr");
    client
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
