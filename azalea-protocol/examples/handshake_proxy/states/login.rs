use std::net::SocketAddr;

use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::client_intention_packet::ClientIntentionPacket,
        login::{ClientboundLoginPacket, ServerboundLoginPacket},
    },
    read::ReadPacketError,
};
use log::{error, info, warn};

use crate::proxy;

/// Wait for the client to send the `Hello` packet,
/// log their username and uuid, and then forward the
/// connection along to the proxy target.
pub async fn handle(
    mut conn: Connection<ServerboundLoginPacket, ClientboundLoginPacket>,
    client_addr: SocketAddr,
    target_addr: SocketAddr,
    intent: ClientIntentionPacket,
) -> anyhow::Result<()> {
    loop {
        match conn.read().await {
            Ok(ServerboundLoginPacket::Hello(packet)) => {
                info!(
                    "Player `{0}` from {1} logging in with uuid: {2}",
                    packet.name,
                    client_addr.ip(),
                    packet.profile_id.to_string()
                );

                // Forward the connection to the proxy target
                proxy::spawn(conn, target_addr, intent, packet);

                break;
            }
            Ok(_) => {
                warn!("Client sent unexpected packet during login!");
            }
            Err(e) => match *e {
                ReadPacketError::ConnectionClosed => {
                    break;
                }

                e => {
                    error!("Error reading clinet login packets: {e}");
                    return Err(e.into());
                }
            },
        }
    }

    Ok(())
}
