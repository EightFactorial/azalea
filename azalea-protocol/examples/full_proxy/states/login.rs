use std::net::SocketAddr;

use azalea_auth::game_profile::GameProfile;
use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{client_intention_packet::ClientIntentionPacket, ServerboundHandshakePacket},
        login::{
            clientbound_login_disconnect_packet::ClientboundLoginDisconnectPacket,
            ClientboundLoginPacket, ServerboundLoginPacket,
        },
    },
};
use log::{error, info};

use crate::proxy::{
    self,
    wrapper::{ClientWrapper, TargetWrapper},
};

/// Reply with the proxy server information
pub async fn handle(
    mut conn: Connection<ServerboundLoginPacket, ClientboundLoginPacket>,
    intent: ClientIntentionPacket,
    client_addr: SocketAddr,
    target_addr: SocketAddr,
) -> anyhow::Result<()> {
    // Connect to the target server
    let mut target_conn = Connection::new(&target_addr).await?;
    target_conn
        .write(ServerboundHandshakePacket::ClientIntention(intent))
        .await?;
    let mut target_conn = target_conn.login();

    // Store the player's profile information
    let mut profile = GameProfile::default();

    loop {
        tokio::select! {
            packet = conn.read() => {
                match packet {
                    Err(e) =>  return Err(handle_error(e.into(), &profile)),
                    Ok(packet) => match handle_client_packet(packet, &mut conn, &mut target_conn, &mut profile, client_addr).await {
                        Err(e) => return Err(handle_error(e, &profile)),
                        Ok(None) => {}
                        Ok(Some(())) => break,
                    }
                }
            },
            packet = target_conn.read() => {
                match packet {
                    Err(e) => return Err(handle_error(e.into(), &profile)),
                    Ok(packet) => match handle_target_packet(packet, &mut conn, &mut target_conn, &mut profile).await {
                        Err(e) => return Err(handle_error(e, &profile)),
                        Ok(None) => {}
                        Ok(Some(())) => break,
                    }
                }
            }
        }
    }

    tokio::spawn(proxy::proxy(
        ClientWrapper::Configuration(conn.configuration()),
        TargetWrapper::Configuration(target_conn.configuration()),
        profile,
    ));

    Ok(())
}

/// Handle an error during login
#[inline]
fn handle_error(e: anyhow::Error, profile: &GameProfile) -> anyhow::Error {
    let name = if profile.name.is_empty() {
        "client".to_string()
    } else {
        format!("`{}`", profile.name)
    };
    error!("Error during login for {name}: {e}");

    e
}

/// Handle a packet from the client
///
/// Returns `Ok(Some(()))` if the login process is complete
async fn handle_client_packet(
    packet: ServerboundLoginPacket,
    client_conn: &mut Connection<ServerboundLoginPacket, ClientboundLoginPacket>,
    target_conn: &mut Connection<ClientboundLoginPacket, ServerboundLoginPacket>,
    profile: &mut GameProfile,
    client_addr: SocketAddr,
) -> anyhow::Result<Option<()>> {
    match packet {
        ServerboundLoginPacket::Hello(packet) => {
            info!(
                "Player \'{0}\' from {1} logging in with uuid: {2}",
                packet.name,
                client_addr.ip(),
                packet.profile_id.to_string()
            );

            profile.name = packet.name.clone();
            profile.uuid = packet.profile_id;

            target_conn
                .write(ServerboundLoginPacket::Hello(packet))
                .await?;
        }
        ServerboundLoginPacket::Key(packet) => {
            let key_bytes = packet.key_bytes.clone();

            target_conn
                .write(ServerboundLoginPacket::Key(packet.clone()))
                .await?;

            client_conn.set_encryption_key(key_bytes.clone().try_into().unwrap());
            target_conn.set_encryption_key(key_bytes.try_into().unwrap());
        }
        ServerboundLoginPacket::LoginAcknowledged(packet) => {
            target_conn
                .write(ServerboundLoginPacket::LoginAcknowledged(packet))
                .await?;

            info!("Player \'{0}\' has logged in!", profile.name);

            return Ok(Some(()));
        }
        packet => {
            target_conn.write(packet).await?;
        }
    }

    Ok(None)
}

/// Handle a packet from the target
///
/// Returns `Ok(Some(()))` if the login process is complete
async fn handle_target_packet(
    packet: ClientboundLoginPacket,
    client_conn: &mut Connection<ServerboundLoginPacket, ClientboundLoginPacket>,
    target_conn: &mut Connection<ClientboundLoginPacket, ServerboundLoginPacket>,
    profile: &mut GameProfile,
) -> anyhow::Result<Option<()>> {
    info!("Packet from target: {:?}", packet);

    match packet {
        ClientboundLoginPacket::Hello(_) => {
            client_conn
                .write(ClientboundLoginPacket::LoginDisconnect(
                    ClientboundLoginDisconnectPacket {
                        reason: "Proxy does not support online servers".into(),
                    },
                ))
                .await?;

            return Err(anyhow::anyhow!("Proxy does not support online servers"));
        }
        ClientboundLoginPacket::LoginDisconnect(packet) => {
            client_conn
                .write(ClientboundLoginPacket::LoginDisconnect(packet.clone()))
                .await?;

            return Err(anyhow::anyhow!(packet.reason.to_string()));
        }
        ClientboundLoginPacket::LoginCompression(packet) => {
            let threshold = packet.compression_threshold;
            target_conn.set_compression_threshold(threshold);

            client_conn
                .write(ClientboundLoginPacket::LoginCompression(packet))
                .await?;
            client_conn.set_compression_threshold(threshold);
        }
        ClientboundLoginPacket::GameProfile(packet) => {
            *profile = packet.game_profile.clone();

            client_conn
                .write(ClientboundLoginPacket::GameProfile(packet))
                .await?;
        }
        packet => {
            client_conn.write(packet).await?;
        }
    }

    Ok(None)
}
