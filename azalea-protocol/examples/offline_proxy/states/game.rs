use azalea_auth::game_profile::GameProfile;
use azalea_protocol::packets::{
    configuration::serverbound_client_information_packet::ClientInformation,
    game::{
        clientbound_disconnect_packet::ClientboundDisconnectPacket,
        clientbound_system_chat_packet::ClientboundSystemChatPacket, ClientboundGamePacket,
        ServerboundGamePacket,
    },
};
use log::info;

use crate::proxy::{
    commands::{Command, OptionalCommand},
    wrapper::{ClientWrapper, ClientboundPacketWrapper, ServerboundPacketWrapper, TargetWrapper},
    ConnType,
};

/// Process packets sent from the client to the target server
pub async fn game_client_to_target(
    packet: ServerboundGamePacket,
    client_profile: &GameProfile,
    client_information: &mut ClientInformation,
    target_conn: &mut TargetWrapper,
    client_conn: &mut ClientWrapper,
) -> anyhow::Result<Option<ConnType>> {
    match packet {
        // Update the client information
        ServerboundGamePacket::ClientInformation(packet) => {
            *client_information = packet.information.clone();

            target_conn
                .write(ServerboundPacketWrapper::Game(
                    ServerboundGamePacket::ClientInformation(packet),
                ))
                .await?;
        }
        // Log chat messages
        ServerboundGamePacket::Chat(packet) => {
            info!("{}: {}", client_profile.name, packet.message);

            target_conn
                .write(ServerboundPacketWrapper::Game(ServerboundGamePacket::Chat(
                    packet,
                )))
                .await?;
        }
        // Log commands
        ServerboundGamePacket::ChatCommand(packet) => {
            info!("{}: /{}", client_profile.name, packet.command);

            // Try to parse a proxy command
            match OptionalCommand::parse_packet(packet) {
                // Handle proxy command
                OptionalCommand::Some(command) => {
                    info!(
                        "Player `{}` executed proxy command: {command:?}",
                        client_profile.name
                    );

                    match command {
                        // Send a disconnect packet to the client
                        Command::Disconnect(reason) => {
                            client_conn
                                .write(ClientboundPacketWrapper::Game(
                                    ClientboundGamePacket::Disconnect(
                                        ClientboundDisconnectPacket {
                                            reason: reason.into(),
                                        },
                                    ),
                                ))
                                .await?;
                        }
                        // Send the message back to the client
                        Command::Echo(message) => {
                            client_conn
                                .write(ClientboundPacketWrapper::Game(
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
                // Forward the command packet to the server
                OptionalCommand::None(packet) => {
                    target_conn
                        .write(ServerboundPacketWrapper::Game(
                            ServerboundGamePacket::ChatCommand(packet),
                        ))
                        .await?;
                }
            }
        }
        // Forward all other packets to the server
        packet => {
            target_conn
                .write(ServerboundPacketWrapper::Game(packet))
                .await?;
        }
    }

    Ok(None)
}

/// Process packets sent from the target server to the client
pub async fn game_target_to_client(
    packet: ClientboundGamePacket,
    client_conn: &mut ClientWrapper,
    _target_conn: &mut TargetWrapper,
) -> anyhow::Result<Option<ConnType>> {
    match packet {
        // Change the connection type
        ClientboundGamePacket::StartConfiguration(packet) => {
            client_conn
                .write(ClientboundPacketWrapper::Game(
                    ClientboundGamePacket::StartConfiguration(packet),
                ))
                .await?;

            return Ok(Some(ConnType::Configuration));
        }
        // Forward other packets to the client
        packet => {
            client_conn
                .write(ClientboundPacketWrapper::Game(packet))
                .await?;
        }
    }

    Ok(None)
}
