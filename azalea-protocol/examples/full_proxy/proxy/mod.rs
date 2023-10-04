use azalea_auth::game_profile::GameProfile;
use azalea_protocol::packets::{
    configuration::ServerboundConfigurationPacket,
    game::{ClientboundGamePacket, ServerboundGamePacket},
};
use log::{error, info};

use self::wrapper::{ClientWrapper, ClientWrapperPacket, TargetWrapper, TargetWrapperPacket};

pub mod wrapper;

pub async fn proxy(
    mut client_conn: ClientWrapper,
    mut target_conn: TargetWrapper,
    client_profile: GameProfile,
) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            packet = client_conn.read() => match packet {
                Err(e) => return Err(handle_error(e.into(), &client_profile)),
                Ok(packet) => match handle_client_packet(packet, &client_profile, &mut target_conn, &mut client_conn).await? {
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
                Err(e) => return Err(handle_error(e.into(), &client_profile)),
                Ok(packet) => match handle_target_packet(packet, &mut client_conn, &mut target_conn).await? {
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

/// Handle an error during proxying
#[inline]
fn handle_error(e: anyhow::Error, profile: &GameProfile) -> anyhow::Error {
    error!("Error proxying packets for `{}`: {e}", profile.name);
    e
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnType {
    Configuration,
    Game,
}

async fn handle_client_packet(
    packet: ClientWrapperPacket,
    client_profile: &GameProfile,
    target_conn: &mut TargetWrapper,
    _client_conn: &mut ClientWrapper,
) -> anyhow::Result<Option<ConnType>> {
    match packet {
        ClientWrapperPacket::Configuration(packet) => {
            match packet {
                ServerboundConfigurationPacket::ClientInformation(packet) => {
                    // TODO: Store the client information

                    target_conn
                        .write(ClientWrapperPacket::Configuration(
                            ServerboundConfigurationPacket::ClientInformation(packet),
                        ))
                        .await?;
                }
                ServerboundConfigurationPacket::FinishConfiguration(packet) => {
                    target_conn
                        .write(ClientWrapperPacket::Configuration(
                            ServerboundConfigurationPacket::FinishConfiguration(packet),
                        ))
                        .await?;

                    return Ok(Some(ConnType::Game));
                }
                _ => {
                    target_conn
                        .write(ClientWrapperPacket::Configuration(packet))
                        .await?;
                }
            }
        }
        ClientWrapperPacket::Game(packet) => {
            match &packet {
                ServerboundGamePacket::Chat(packet) => {
                    info!("{}: {}", client_profile.name, packet.message);
                }
                ServerboundGamePacket::ChatCommand(packet) => {
                    info!("{}: /{}", client_profile.name, packet.command);
                }
                _ => {}
            }

            target_conn.write(ClientWrapperPacket::Game(packet)).await?;
        }
    }

    Ok(None)
}

async fn handle_target_packet(
    packet: TargetWrapperPacket,
    client_conn: &mut ClientWrapper,
    _target_conn: &mut TargetWrapper,
) -> anyhow::Result<Option<ConnType>> {
    match packet {
        TargetWrapperPacket::Configuration(packet) => {
            client_conn
                .write(TargetWrapperPacket::Configuration(packet))
                .await?;
        }
        TargetWrapperPacket::Game(packet) => match packet {
            ClientboundGamePacket::StartConfiguration(packet) => {
                client_conn
                    .write(TargetWrapperPacket::Game(
                        ClientboundGamePacket::StartConfiguration(packet),
                    ))
                    .await?;

                return Ok(Some(ConnType::Configuration));
            }
            _ => {
                client_conn.write(TargetWrapperPacket::Game(packet)).await?;
            }
        },
    }

    Ok(None)
}
