//! A proxy server that fully decodes and re-encodes packets between the client
//! and target. This can be used to modify packets, but only works for offline
//! mode servers.

use std::net::SocketAddr;

use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{ClientboundHandshakePacket, ServerboundHandshakePacket},
        ConnectionProtocol,
    },
};
use log::{error, info, warn};
use tokio::net::{TcpListener, TcpStream};
use tracing::Level;

mod proxy;
mod states;

// The address and port to listen on
const LISTEN_ADDR: &str = "127.0.0.1:25566";

// The address and port of the proxy target
const PROXY_ADDR: &str = "127.0.0.1:25565";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .without_time()
        .init();

    // Bind to an address and port
    let listener = match option_env!("LISTEN_ADDR") {
        Some(addr) => TcpListener::bind(addr).await?,
        None => TcpListener::bind(LISTEN_ADDR).await?,
    };

    // Get the target address
    let target: SocketAddr = match option_env!("PROXY_ADDR") {
        Some(addr) => addr.parse()?,
        None => PROXY_ADDR.parse()?,
    };

    loop {
        // When a connection is made, pass it off to another thread
        let (stream, _) = listener.accept().await?;

        // Set nodelay and spawn a new thread
        if let Err(e) = stream.set_nodelay(true) {
            error!("Failed to set nodelay: {e}");
        } else {
            tokio::spawn(handle_connection(stream, target));
        }
    }
}

async fn handle_connection(stream: TcpStream, target_addr: SocketAddr) -> anyhow::Result<()> {
    // Get the ip address of the connecting client
    let Ok(client_addr) = stream.peer_addr() else {
        error!(target: "offline_proxy::incoming", "Failed to get ip address of client");
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
                target: "offline_proxy::incoming",
                "New connection from {0}, Version: {1}, Intent: {2}",
                client_addr.ip(),
                packet.protocol_version,
                packet.intention
            );

            // Return the packet
            packet
        }
        Err(e) => {
            let e = e.into();
            warn!(target: "offline_proxy::incoming", "Error reading client intent: {e}");
            return Err(e);
        }
    };

    match intent.intention {
        ConnectionProtocol::Status => {
            // Handle the status request
            states::status(conn.status(), intent, target_addr).await?;
        }
        ConnectionProtocol::Login => {
            // Handle the login request
            states::login(conn.login(), intent, client_addr, target_addr).await?;
        }
        intent => {
            warn!(target: "offline_proxy::incoming", "Client provided weird intent: {intent}");
        }
    }

    Ok(())
}