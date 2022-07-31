use bevy::{prelude::*, log::LogPlugin};
use bevy_punchthrough::server::PunchThroughServerPlugin;


fn main(){
    let mut app = bevy::app::App::new();

    app.add_plugins(MinimalPlugins);
    app.add_plugin(LogPlugin);
    app.add_plugin(PunchThroughServerPlugin{port: 5000});
    app.add_startup_system(server_start);

    app.run();
}

fn server_start(){
    info!("PunchThrough Server Starting")
}