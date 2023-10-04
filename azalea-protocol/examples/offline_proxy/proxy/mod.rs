use azalea_auth::game_profile::GameProfile;
use azalea_protocol::packets::{
    configuration::{
        serverbound_client_information_packet::ClientInformation, ServerboundConfigurationPacket,
    },
    game::{
        clientbound_disconnect_packet::ClientboundDisconnectPacket,
        clientbound_system_chat_packet::ClientboundSystemChatPacket, ClientboundGamePacket,
        ServerboundGamePacket,
    },
};
use log::{error, info};

use commands::{Command, OptionalCommand};
use wrapper::{ClientWrapper, ClientWrapperPacket, TargetWrapper, TargetWrapperPacket};

mod commands;
pub mod wrapper;

/// Proxy packets between the client and target,
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
                Err(e) => return Err(handle_error(e.into(), &client_profile)),
                Ok(packet) => match handle_client_packet(packet, &client_profile, &mut client_information, &mut target_conn, &mut client_conn).await? {
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

/// Used as a signal to change the connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnType {
    Configuration,
    Game,
}

/// Handle packets from the client
async fn handle_client_packet(
    packet: ClientWrapperPacket,
    client_profile: &GameProfile,
    client_information: &mut ClientInformation,
    target_conn: &mut TargetWrapper,
    client_conn: &mut ClientWrapper,
) -> anyhow::Result<Option<ConnType>> {
    match packet {
        // Forward configuration packets from the client to the target
        ClientWrapperPacket::Configuration(packet) => match packet {
            // Update the client information
            ServerboundConfigurationPacket::ClientInformation(packet) => {
                *client_information = packet.information.clone();

                target_conn
                    .write(ClientWrapperPacket::Configuration(
                        ServerboundConfigurationPacket::ClientInformation(packet),
                    ))
                    .await?;
            }
            // Change the connection type
            ServerboundConfigurationPacket::FinishConfiguration(packet) => {
                target_conn
                    .write(ClientWrapperPacket::Configuration(
                        ServerboundConfigurationPacket::FinishConfiguration(packet),
                    ))
                    .await?;

                return Ok(Some(ConnType::Game));
            }
            packet => {
                target_conn
                    .write(ClientWrapperPacket::Configuration(packet))
                    .await?;
            }
        },
        // Forward game packets from the client to the target
        ClientWrapperPacket::Game(packet) => {
            match packet {
                // Update the client information
                ServerboundGamePacket::ClientInformation(packet) => {
                    *client_information = packet.information.clone();

                    target_conn
                        .write(ClientWrapperPacket::Game(
                            ServerboundGamePacket::ClientInformation(packet),
                        ))
                        .await?;
                }
                // Log chat messages
                ServerboundGamePacket::Chat(packet) => {
                    info!("{}: {}", client_profile.name, packet.message);

                    target_conn
                        .write(ClientWrapperPacket::Game(ServerboundGamePacket::Chat(
                            packet,
                        )))
                        .await?;
                }
                // Log commands
                ServerboundGamePacket::ChatCommand(packet) => {
                    info!("{}: /{}", client_profile.name, packet.command);

                    // Try to parse a proxy command
                    match commands::parse_command(packet) {
                        // Handle proxy command
                        Ok(OptionalCommand::Some(command)) => {
                            info!(
                                "Player `{}` executed proxy command: {command:?}",
                                client_profile.name
                            );

                            match command {
                                // Send a disconnect packet to the client
                                Command::Disconnect(reason) => {
                                    client_conn
                                        .write(TargetWrapperPacket::Game(
                                            ClientboundGamePacket::Disconnect(
                                                ClientboundDisconnectPacket {
                                                    reason: reason.into(),
                                                },
                                            ),
                                        ))
                                        .await?;
                                }
                                // Send a chat message to the client
                                Command::Echo(message) => {
                                    client_conn
                                        .write(TargetWrapperPacket::Game(
                                            ClientboundGamePacket::SystemChat(
                                                ClientboundSystemChatPacket {
                                                    content: message.into(),
                                                    overlay: false,
                                                },
                                            ),
                                        ))
                                        .await?;
                                }
                            }
                        }
                        // Forward the command to the server
                        Ok(OptionalCommand::None(packet)) => {
                            target_conn
                                .write(ClientWrapperPacket::Game(
                                    ServerboundGamePacket::ChatCommand(packet),
                                ))
                                .await?;
                        }
                        Err(e) => {
                            error!("Error proxying command for `{}`: {e}", client_profile.name);
                            return Err(e);
                        }
                    }
                }
                packet => {
                    target_conn.write(ClientWrapperPacket::Game(packet)).await?;
                }
            }
        }
    }

    Ok(None)
}

/// Handle packets from the target
async fn handle_target_packet(
    packet: TargetWrapperPacket,
    client_conn: &mut ClientWrapper,
    _target_conn: &mut TargetWrapper,
) -> anyhow::Result<Option<ConnType>> {
    match packet {
        // Forward configuration packets from the target to the client
        TargetWrapperPacket::Configuration(packet) => {
            client_conn
                .write(TargetWrapperPacket::Configuration(packet))
                .await?;
        }
        // Forward game packets from the target to the client
        TargetWrapperPacket::Game(packet) => match packet {
            // Change the connection type
            ClientboundGamePacket::StartConfiguration(packet) => {
                client_conn
                    .write(TargetWrapperPacket::Game(
                        ClientboundGamePacket::StartConfiguration(packet),
                    ))
                    .await?;

                return Ok(Some(ConnType::Configuration));
            }
            packet => {
                client_conn.write(TargetWrapperPacket::Game(packet)).await?;
            }
        },
    }

    Ok(None)
}
