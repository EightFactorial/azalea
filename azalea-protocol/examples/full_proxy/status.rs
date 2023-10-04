use std::net::SocketAddr;

use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{client_intention_packet::ClientIntentionPacket, ServerboundHandshakePacket},
        status::{ClientboundStatusPacket, ServerboundStatusPacket},
    },
    read::ReadPacketError,
};
use log::{error, info};

/// Reply with the proxy server information
pub async fn handle(
    mut conn: Connection<ServerboundStatusPacket, ClientboundStatusPacket>,
    intent: ClientIntentionPacket,
    client_ip: SocketAddr,
    target_ip: SocketAddr,
) -> anyhow::Result<()> {
    // Connect to the target server
    let mut target_conn = Connection::new(&target_ip).await?;
    target_conn
        .write(ServerboundHandshakePacket::ClientIntention(intent))
        .await?;
    let mut target_conn = target_conn.status();

    loop {
        tokio::select! {
            // Read packets from the client and forward them to the target
            result = conn.read() => {
                match result {
                    Ok(ServerboundStatusPacket::StatusRequest(packet)) => {
                        info!("Forwarding status request from {0}", client_ip.ip());

                        target_conn
                            .write(ServerboundStatusPacket::StatusRequest(packet))
                            .await?;
                    }
                    Ok(ServerboundStatusPacket::PingRequest(packet)) => {
                        info!("Forwarding ping request from {0}", client_ip.ip());

                        target_conn
                            .write(ServerboundStatusPacket::PingRequest(packet))
                            .await?;
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
            // Read packets from the target and forward them to the client
            result = target_conn.read() => {
                match result {
                    Ok(ClientboundStatusPacket::StatusResponse(packet)) => {
                        info!("Got status response for {0}", client_ip.ip());

                        conn
                            .write(ClientboundStatusPacket::StatusResponse(packet))
                            .await?;
                    }
                    Ok(ClientboundStatusPacket::PongResponse(packet)) => {
                        info!("Got pong response for {0}", client_ip.ip());

                        conn
                            .write(ClientboundStatusPacket::PongResponse(packet))
                            .await?;
                    }
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
