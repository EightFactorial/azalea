use azalea_protocol::{
    connect::Connection,
    packets::status::{
        clientbound_pong_response_packet::ClientboundPongResponsePacket,
        clientbound_status_response_packet::ClientboundStatusResponsePacket,
        ClientboundStatusPacket, ServerboundStatusPacket,
    },
    read::ReadPacketError,
};
use log::error;

use crate::{PROXY_DESC, PROXY_FAVICON, PROXY_PLAYERS, PROXY_SECURE_CHAT, PROXY_VERSION};

/// Reply with the proxy server information
pub async fn status(
    mut conn: Connection<ServerboundStatusPacket, ClientboundStatusPacket>,
) -> anyhow::Result<()> {
    loop {
        match conn.read().await {
            Ok(ServerboundStatusPacket::StatusRequest(_)) => {
                // Respond with the proxy server information
                conn.write(
                    ClientboundStatusResponsePacket {
                        description: PROXY_DESC.into(),
                        favicon: PROXY_FAVICON.clone(),
                        players: PROXY_PLAYERS.clone(),
                        version: PROXY_VERSION.clone(),
                        enforces_secure_chat: PROXY_SECURE_CHAT,
                    }
                    .get(),
                )
                .await?;
            }
            Ok(ServerboundStatusPacket::PingRequest(packet)) => {
                // Respond with the same time as the client
                conn.write(ClientboundPongResponsePacket { time: packet.time }.get())
                    .await?;

                // Close the connection
                break;
            }
            Err(e) => match *e {
                ReadPacketError::ConnectionClosed => {
                    break;
                }

                e => {
                    error!("Error reading client status packets: {e}");
                    return Err(e.into());
                }
            },
        }
    }

    Ok(())
}
