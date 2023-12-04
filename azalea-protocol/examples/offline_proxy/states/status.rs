use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{client_intention_packet::ClientIntentionPacket, ServerboundHandshakePacket},
        status::{ClientboundStatusPacket, ServerboundStatusPacket},
    },
    read::ReadPacketError,
    resolver, ServerAddress,
};
use tracing::error;

/// Reply with the proxy server information
pub async fn status(
    mut conn: Connection<ServerboundStatusPacket, ClientboundStatusPacket>,
    mut intent: ClientIntentionPacket,
    target_addr: ServerAddress,
) -> anyhow::Result<()> {
    // Resolve and modify intent for target
    let resolved_address = resolver::resolve_address(&target_addr).await?;
    intent.hostname = target_addr.host.clone();
    intent.port = target_addr.port;

    // Connect to the target server
    let mut target_conn = Connection::new(&resolved_address).await?;
    target_conn
        .write(ServerboundHandshakePacket::ClientIntention(intent))
        .await?;
    let mut target_conn = target_conn.status();

    loop {
        tokio::select! {
            // Read packets from the client and forward them to the target
            result = conn.read() => {
                match result {
                    Ok(packet) => target_conn.write(packet).await?,
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
            // Read packets from the target and forward them to the client
            result = target_conn.read() => {
                match result {
                    Ok(packet) => conn.write(packet).await?,
                    Err(e) => match *e {
                        ReadPacketError::ConnectionClosed => {
                            break;
                        }

                        e => {
                            error!("Error reading target status packets: {e}");
                            return Err(e.into());
                        }
                    },
                }
            }
        }
    }

    Ok(())
}
