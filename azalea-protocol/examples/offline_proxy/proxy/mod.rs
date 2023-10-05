use azalea_auth::game_profile::GameProfile;
use azalea_protocol::packets::configuration::serverbound_client_information_packet::ClientInformation;
use log::error;

use wrapper::{ClientWrapper, ClientboundPacketWrapper, ServerboundPacketWrapper, TargetWrapper};

use crate::states::{
    config_client_to_target, config_target_to_client, game_client_to_target, game_target_to_client,
};

pub mod commands;
pub mod wrapper;

/// Used as a signal to change the connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnType {
    Configuration,
    Game,
}

/// Proxy packets between the client and server,
/// possibly changing the connection type.
pub async fn proxy(
    mut client_conn: ClientWrapper,
    mut target_conn: TargetWrapper,
    client_profile: GameProfile,
) -> anyhow::Result<()> {
    let mut client_information = ClientInformation::default();

    loop {
        tokio::select! {
            packet = client_conn.read() => match packet {
                Err(e) => {
                    error!("Error proxying c2s packets for `{}`: {e}", client_profile.name);
                    return Err(e.into());
                },
                Ok(packet) => match match packet {
                    // Forward configuration packets from the client to the server
                    ServerboundPacketWrapper::Configuration(packet) => {
                        config_client_to_target(
                            packet,
                            &client_profile,
                            &mut client_information,
                            &mut target_conn,
                            &mut client_conn,
                        )
                        .await
                    }
                    // Forward game packets from the client to the server
                    ServerboundPacketWrapper::Game(packet) => {
                        game_client_to_target(
                            packet,
                            &client_profile,
                            &mut client_information,
                            &mut target_conn,
                            &mut client_conn,
                        )
                        .await
                    }
                }? {
                    None => {}
                    // Change the connection type
                    Some(conn_type) => match conn_type {
                        ConnType::Configuration => {
                            client_conn = client_conn.configuration();
                            target_conn = target_conn.configuration();
                        }
                        ConnType::Game => {
                            client_conn = client_conn.game();
                            target_conn = target_conn.game();
                        }
                    }
                },
            },
            packet = target_conn.read() => match packet {
                Err(e) => {
                    error!("Error proxying s2c packets for `{}`: {e}", client_profile.name);
                    return Err(e.into());
                },
                Ok(packet) => match match packet {
                    // Forward configuration packets from the server to the client
                    ClientboundPacketWrapper::Configuration(packet) => {
                        config_target_to_client(packet, &mut client_conn, &mut target_conn).await
                    }
                    // Forward game packets from the server to the client
                    ClientboundPacketWrapper::Game(packet) => {
                        game_target_to_client(packet, &mut client_conn, &mut target_conn).await
                    }
                }? {
                    None => {}
                    // Change the connection type
                    Some(conn_type) => match conn_type  {
                        ConnType::Configuration => {
                            client_conn = client_conn.configuration();
                            target_conn = target_conn.configuration();
                        }
                        ConnType::Game => {
                            client_conn = client_conn.game();
                            target_conn = target_conn.game();
                        }
                    }
                }
            },
        }
    }
}
