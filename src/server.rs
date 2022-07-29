use std::{net::{UdpSocket, IpAddr}, time::SystemTime, collections::HashMap};

use bevy::{prelude::*};
use bevy_renet::{
    renet::{RenetServer, ServerConfig, RenetConnectionConfig, ServerAuthentication, ServerEvent},
    RenetServerPlugin,
};

use crate::{PROTOCOL_ID, ServerChannel, ClientChannel, ClientHostCommand};

struct ClientHost{
    ip: IpAddr,
    port: u16,
    last_heartbeat: u128
}

struct PunchThroughServerRes {
    pub server: RenetServer,
    pub hosts: HashMap<IpAddr, ClientHost>
}
pub struct PunchThroughServerPlugin;

impl Plugin for PunchThroughServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RenetServerPlugin);
        app.add_startup_system(init_server);
        app.insert_resource(PunchThroughServerRes {
            server: get_server(),
            hosts: HashMap::new(),
        });
        app.add_system(process_server_events);

    }
}

fn init_server() {
    println!("Initializing Server");
}

fn process_server_events(mut server_events: EventReader<ServerEvent>, mut server_res: ResMut<PunchThroughServerRes>){
    for server_event in server_events.iter(){
        match server_event{
            ServerEvent::ClientConnected(id, _) => {
                println!("Client connected: {}", id);
            },
            ServerEvent::ClientDisconnected(id) => {
                println!("Client disconnected: {}", id);
            }   
        }
    }

    //Parse messages from the clients
    let punchthrough_res = server_res.as_mut();
    for client_id in punchthrough_res.server.clients_id().into_iter() {
        while let Some(message) = server_res.as_mut().server.receive_message(client_id, ClientChannel::Command.id()) {
            let cmd: ClientHostCommand = match bincode::deserialize(&message)
            {
                Ok(cmd) => cmd, 
                Err(e) => {
                    println!("Error deserializing client host command {e:#?}");
                    continue
                }
            };
        }
    }

}

fn get_server() -> RenetServer {
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind(server_addr).unwrap();
    let connection_config = server_connection_config();
    let server_config =
        ServerConfig::new(64, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure);
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    RenetServer::new(current_time, server_config, connection_config, socket).unwrap()
}

pub fn server_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ServerChannel::channels_config(),
        receive_channels_config: ClientChannel::channels_config(),
        ..Default::default()
    }
}
