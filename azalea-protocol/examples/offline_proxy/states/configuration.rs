use azalea_auth::game_profile::GameProfile;
use azalea_protocol::packets::configuration::{
    serverbound_client_information_packet::ClientInformation, ClientboundConfigurationPacket,
    ServerboundConfigurationPacket,
};

use crate::proxy::{
    wrapper::{ClientWrapper, ClientboundPacketWrapper, ServerboundPacketWrapper, TargetWrapper},
    ConnType,
};

/// Forward configuration packets from the client to the server
///
/// These packets contain of the actions of the connected client
pub async fn config_client_to_target(
    packet: ServerboundConfigurationPacket,
    _client_profile: &GameProfile,
    client_information: &mut ClientInformation,
    target_conn: &mut TargetWrapper,
    _client_conn: &mut ClientWrapper,
) -> anyhow::Result<Option<ConnType>> {
    match packet {
        // Update the client information
        ServerboundConfigurationPacket::ClientInformation(packet) => {
            *client_information = packet.information.clone();

            target_conn
                .write(ServerboundPacketWrapper::Configuration(
                    ServerboundConfigurationPacket::ClientInformation(packet),
                ))
                .await?;
        }
        // Change the connection type
        ServerboundConfigurationPacket::FinishConfiguration(packet) => {
            target_conn
                .write(ServerboundPacketWrapper::Configuration(
                    ServerboundConfigurationPacket::FinishConfiguration(packet),
                ))
                .await?;

            return Ok(Some(ConnType::Game));
        }
        // Forward other packets to the target
        packet => {
            target_conn
                .write(ServerboundPacketWrapper::Configuration(packet))
                .await?;
        }
    }

    Ok(None)
}

/// Forward configuration packets from the server to the client
///
/// There's no interesting data in these packets, so we just forward them
pub async fn config_target_to_client(
    packet: ClientboundConfigurationPacket,
    client_conn: &mut ClientWrapper,
    _target_conn: &mut TargetWrapper,
) -> anyhow::Result<Option<ConnType>> {
    client_conn
        .write(ClientboundPacketWrapper::Configuration(packet))
        .await?;

    Ok(None)
}
