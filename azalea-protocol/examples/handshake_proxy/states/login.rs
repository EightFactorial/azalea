use std::net::SocketAddr;

use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::client_intention_packet::ClientIntentionPacket,
        login::{ClientboundLoginPacket, ServerboundLoginPacket},
    },
    read::ReadPacketError,
};
use futures_util::FutureExt;
use log::{error, info, warn};

use crate::proxy;

/// Wait for the client to send the `Hello` packet,
/// log their username and uuid, and then forward the
/// connection along to the proxy target.
pub async fn login(
    mut conn: Connection<ServerboundLoginPacket, ClientboundLoginPacket>,
    client_addr: SocketAddr,
    target_addr: SocketAddr,
    intent: ClientIntentionPacket,
) -> anyhow::Result<()> {
    loop {
        match conn.read().await {
            // This should be the first packet sent by the client
            Ok(ServerboundLoginPacket::Hello(packet)) => {
                info!(
                    "Player `{0}` ({1}) logging in from {2}",
                    packet.name,
                    packet.profile_id.to_string(),
                    client_addr.ip()
                );

                // Unwrap the connection
                let inbound = match conn.unwrap() {
                    Ok(inbound) => inbound,
                    Err(e) => {
                        error!("Failed to unwrap connection: {e}");
                        return Err(e.into());
                    }
                };

                // Spawn a task to handle the proxy connection
                tokio::spawn(
                    proxy::proxy(inbound, target_addr, intent, packet).map(|result| {
                        if let Err(e) = result {
                            error!("Failed to proxy: {e}");
                        }
                    }),
                );

                return Ok(());
            }
            Ok(_) => {
                warn!("Client sent unexpected packet during login!");
            }
            Err(e) => match *e {
                ReadPacketError::ConnectionClosed => {
                    return Ok(());
                }

                e => {
                    error!("Error reading client login packets: {e}");
                    return Err(e.into());
                }
            },
        }
    }
}
