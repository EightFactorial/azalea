//! A "simple" server that gets login information and proxies connections.
//! After login all connections are encrypted and Azalea cannot read them.

use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{ClientboundHandshakePacket, ServerboundHandshakePacket},
        status::clientbound_status_response_packet::{Players, Version},
        ConnectionProtocol, PROTOCOL_VERSION,
    },
};
use log::{error, info, warn};
use once_cell::sync::Lazy;
use tokio::net::{TcpListener, TcpStream};
use tracing::Level;

mod login;
mod proxy;
mod status;

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
            error!(target: "handshake_proxy::incoming", "Failed to set nodelay: {e}");
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
        ConnectionProtocol::Status => {
            // Handle the status request
            status::handle(conn.status(), ip).await?;
        }
        ConnectionProtocol::Login => {
            // Handle the login request
            login::handle(conn.login(), ip, intent).await?;
        }
        intent => {
            warn!(target: "handshake_proxy::incoming", "Client provided weird intent: {intent}");
        }
    }

    Ok(())
}
