use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_renet::{
    renet::{RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::{ClientChannel, ClientHostMessage, ServerChannel, PROTOCOL_ID, renet_plugin::PTRenetServerPlugin};

pub struct PunchThroughServerRes {
    pub hosts: HashMap<String, (u64, SocketAddr)>,
    pub host_client_idx: HashMap<u64, (String, SocketAddr)>,
}

pub struct PunchThroughServerPlugin{
    pub port: u16
}

impl Plugin for PunchThroughServerPlugin {
    fn build(&self, app: &mut App) {
        info!("Building Plugin");
        app.add_plugin(PTRenetServerPlugin);
        app.insert_resource(PunchThroughServerRes {
            hosts: HashMap::new(),
            host_client_idx: HashMap::new(),
        });
        app.insert_resource(get_server(self.port));
        app.add_system(process_server_events);
        app.add_startup_system(server_plugin_init);
    }
}

fn server_plugin_init(){
    info!("Initializing Server Plugin");
}

fn process_server_events(
    mut server_events: EventReader<ServerEvent>,
    mut server_res: ResMut<PunchThroughServerRes>,
    mut server: ResMut<RenetServer>,
) {
    let pt_res = server_res.as_mut();

    for server_event in server_events.iter() {
        match server_event {
            ServerEvent::ClientConnected(id, _user_data) => {
                let client_addr = server.netcode_server.client_addr(*id).unwrap();
                println!(
                    "Client connected: {} on {}:{}",
                    id,
                    client_addr.ip(),
                    client_addr.port()
                );
            }

            ServerEvent::ClientDisconnected(id) => {
                if pt_res.host_client_idx.contains_key(id) {
                    let host_id = pt_res.host_client_idx.get(id).unwrap();
                    pt_res.hosts.remove(&host_id.0);
                    pt_res.host_client_idx.remove(id);
                }

                println!("Client disconnected: {}", id);
            }
        }
    }

    //Parse messages from the clients
    let punchthrough_res = server_res.as_mut();

    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server
            .receive_message(client_id, ClientChannel::Command.id())
        {
            let cmd: ClientHostMessage = match bincode::deserialize(&message) {
                Ok(cmd) => cmd,
                Err(e) => {
                    println!("Error deserializing client host command {e:#?}");
                    continue;
                }
            };

            match cmd {
                ClientHostMessage::HostNewLobby => {
                    let addr = server
                        .netcode_server
                        .client_addr(client_id)
                        .unwrap();
                    //Construct Host Lobby of LobbyId, SocketAddr
                    let random_str: Vec<u8> =
                        thread_rng().sample_iter(&Alphanumeric).take(5).collect();

                    //TODO: Put code in to ensure this is unique
                    let id = match String::from_utf8(random_str) {
                        Ok(id) => id.to_ascii_uppercase(),
                        Err(e) => {
                            println!("Error generating id: {e:#?}");
                            continue;
                        }
                    };

                    punchthrough_res.hosts.insert(id.clone(), (client_id, addr));
                    let message = bincode::serialize(&ClientHostMessage::NewLobbyResponse {
                        lobby_id: id.clone(),
                    })
                    .expect("Could not encode id to bytes");
                    server.send_message(
                        client_id,
                        ClientChannel::Command.id(),
                        message,
                    )
                }

                ClientHostMessage::RequestSwap { lobby_id } => {
                    if punchthrough_res.hosts.contains_key(&lobby_id.to_ascii_uppercase()) {
                        let message =
                            bincode::serialize(&ClientHostMessage::JoinLobbyResponse { err: None })
                                .expect("Could not deserialize JoinLobbyResponse to bytes.");
                        server.send_message(
                            client_id,
                            ClientChannel::Command.id(),
                            message,
                        );

                        //Send handshake command to client
                        if let Some(socket) = server
                            .netcode_server
                            .client_addr(client_id)
                        {
                            let client_swap_message =
                                bincode::serialize(&ClientHostMessage::AttemptHandshakeCommand {
                                    socket,
                                })
                                .expect("Error serializing Client_Swap_Message to bytes");
                            server.send_message(
                                client_id,
                                ClientChannel::Command.id(),
                                client_swap_message,
                            );

                            //Send handshake command to client client
                            let host_client = punchthrough_res.hosts.get(&lobby_id.to_uppercase()).unwrap();
                            let server_swap_message =
                                bincode::serialize(&ClientHostMessage::AttemptHandshakeCommand {
                                    socket: host_client.1,
                                })
                                .expect("Could not serialize swap message to bytes");
                            server.send_message(
                                host_client.0,
                                ClientChannel::Command.id(),
                                server_swap_message,
                            );
                        } else {
                            let join_response =
                                bincode::serialize(&ClientHostMessage::JoinLobbyResponse {
                                    err: Some(crate::ClientError::LobbyNotFound {
                                        lobby: lobby_id.to_uppercase(),
                                    }),
                                })
                                .expect("Could not serialize client error to bytes");

                            server.send_message(client_id, ClientChannel::Command.id(), join_response);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn get_server(port: u16) -> RenetServer {
    let server_addr = format!("127.0.0.1:{port}").parse().unwrap(); //TODO: Externalize these to CLAP Args
    let socket = UdpSocket::bind(server_addr).unwrap();
    let connection_config = server_connection_config();
    let server_config =
        ServerConfig::new(64, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure);
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let server = RenetServer::new(current_time, server_config, connection_config, socket).unwrap();
    info!("Started Renet server on port {port}");

    server
}

pub fn server_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ServerChannel::channels_config(),
        receive_channels_config: ClientChannel::channels_config(),
        ..Default::default()
    }
}
