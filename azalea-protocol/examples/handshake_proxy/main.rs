//! A "simple" server that gets login information and proxies connections.
//! After login all connections are encrypted and Azalea cannot read them.

use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{ClientboundHandshakePacket, ServerboundHandshakePacket},
        login::ServerboundLoginPacket,
        status::{
            clientbound_pong_response_packet::ClientboundPongResponsePacket,
            clientbound_status_response_packet::{
                ClientboundStatusResponsePacket, Players, Version,
            },
            ServerboundStatusPacket,
        },
        ConnectionProtocol, PROTOCOL_VERSION,
    },
    read::ReadPacketError,
};
use log::{error, info, warn};
use once_cell::sync::Lazy;
use tokio::net::{TcpListener, TcpStream};
use tracing::Level;

mod proxy;

// The address and port to listen on
const LISTEN_ADDR: &str = "127.0.0.1:25566";

// The address and port of the proxy target
const PROXY_ADDR: &str = "127.0.0.1:25565";

// String must be formatted like "data:image/png;base64,<data>"
static PROXY_FAVICON: Lazy<Option<String>> = Lazy::new(|| None);
static PROXY_VERSION: Lazy<Version> = Lazy::new(|| Version {
    name: "1.20.2".to_string(),
    protocol: PROTOCOL_VERSION as i32,
});

const PROXY_DESC: &str = "An Azalea Minecraft Proxy";
const PROXY_SECURE_CHAT: Option<bool> = Some(false);
const PROXY_PLAYERS: Players = Players {
    max: 1,
    online: 0,
    sample: Vec::new(),
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .without_time()
        .init();

    // Bind to an address and port
    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    loop {
        // When a connection is made, pass it off to another thread
        let (stream, _) = listener.accept().await?;

        // Set nodelay and spawn a new thread
        if let Err(e) = stream.set_nodelay(true) {
            error!(target: "handshake_proxy::init", "Failed to set nodelay: {e}");
        } else {
            tokio::spawn(handle_connection(stream));
        }
    }
}

async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    // Get the ip address of the connecting client
    let Ok(ip) = stream.peer_addr() else {
        error!(target: "handshake_proxy::incoming", "Failed to get ip address of client");
        return Ok(());
    };

    // The first packet sent from a client is the intent packet.
    // This specifies whether the client is pinging
    // the server or is going to join the game.
    let mut conn: Connection<ServerboundHandshakePacket, ClientboundHandshakePacket> =
        Connection::wrap(stream);
    let intent = match conn.read().await {
        Ok(ServerboundHandshakePacket::ClientIntention(packet)) => {
            // Log the connection
            info!(
                target: "handshake_proxy::incoming",
                "New connection from {0}, Version: {1}, Intent: {2}",
                ip.ip(),
                packet.protocol_version,
                packet.intention
            );

            // Return the packet
            packet
        }
        Err(e) => {
            let e = e.into();
            warn!(target: "handshake_proxy::incoming", "Error reading client intent: {e}");
            return Err(e);
        }
    };

    match intent.intention {
        // If the client is pinging the proxy,
        // reply with the information below.
        ConnectionProtocol::Status => {
            let mut conn = conn.status();
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

                        info!(target: "handshake_proxy::status", "Sent status to {}", ip.ip());
                    }
                    Ok(ServerboundStatusPacket::PingRequest(packet)) => {
                        // Respond with the same time as the client
                        conn.write(ClientboundPongResponsePacket { time: packet.time }.get())
                            .await?;

                        info!(target: "handshake_proxy::status", "Sent ping to {}", ip.ip());

                        // Close the connection
                        break;
                    }
                    Err(e) => match *e {
                        ReadPacketError::ConnectionClosed => {
                            break;
                        }

                        e => {
                            error!(target: "handshake_proxy::status", "Error reading client status packets: {e}");
                            return Err(e.into());
                        }
                    },
                }
            }
        }
        // If the client intends to join the proxy,
        // wait for them to send the `Hello` packet to
        // log their username and uuid, then forward the
        // connection along to the proxy target.
        ConnectionProtocol::Login => {
            let mut conn = conn.login();
            loop {
                match conn.read().await {
                    Ok(ServerboundLoginPacket::Hello(packet)) => {
                        info!(
                            target: "handshake_proxy::login",
                            "Player \'{0}\' from {1} logging in with uuid: {2}",
                            packet.name,
                            ip.ip(),
                            packet.profile_id.to_string()
                        );

                        proxy::spawn(conn, intent, packet);
                        break;
                    }
                    Ok(_) => {
                        warn!(target: "handshake_proxy::login", "Client sent unexpected packet during login!");
                    }
                    Err(e) => match *e {
                        ReadPacketError::ConnectionClosed => {
                            break;
                        }

                        e => {
                            error!(target: "handshake_proxy::login", "Error reading clinet login packets: {e}");
                            return Err(e.into());
                        }
                    },
                }
            }
        }
        intent => {
            warn!(target: "handshake_proxy::incoming", "Client provided weird intent: {intent}");
        }
    }

    Ok(())
}
