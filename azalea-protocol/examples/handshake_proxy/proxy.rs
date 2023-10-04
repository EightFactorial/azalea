use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{
            client_intention_packet::ClientIntentionPacket, ClientboundHandshakePacket,
            ServerboundHandshakePacket,
        },
        login::{
            serverbound_hello_packet::ServerboundHelloPacket, ClientboundLoginPacket,
            ServerboundLoginPacket,
        },
    },
};
use futures::FutureExt;
use log::{error, info};
use std::error::Error;
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
};

use crate::PROXY_ADDR;

/// Turn the incoming connection into a [TcpStream]
/// and spawn a new thread
#[inline]
pub fn spawn(
    conn: Connection<ServerboundLoginPacket, ClientboundLoginPacket>,
    intent: ClientIntentionPacket,
    hello: ServerboundHelloPacket,
) {
    let Ok(inbound) = conn.unwrap() else {
        error!("Failed to unwrap connection");
        return;
    };

    tokio::spawn(proxy(inbound, intent, hello).map(|result| {
        if let Err(e) = result {
            error!("Failed to proxy: {e}");
        }
    }));
}

/// Create a connection to the proxy target,
/// repeat the packets recieved earlier, and
/// forward data from the connection to the proxy target.
async fn proxy(
    mut inbound: TcpStream,
    intent: ClientIntentionPacket,
    hello: ServerboundHelloPacket,
) -> Result<(), Box<dyn Error>> {
    let outbound = TcpStream::connect(PROXY_ADDR).await?;
    let name = hello.name.clone();
    outbound.set_nodelay(true)?;

    // Repeat the intent and hello packet
    // recieved earlier to the proxy target
    let mut outbound: Connection<ClientboundHandshakePacket, ServerboundHandshakePacket> =
        Connection::wrap(outbound);
    outbound.write(intent.get()).await?;

    let mut outbound = outbound.login();
    outbound.write(hello.get()).await?;
    let mut outbound = outbound.unwrap()?;

    // Split the incoming and outgoing connections in
    // halves and handle each pair on separate threads.
    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = async {
        io::copy(&mut ri, &mut wo).await?;
        wo.shutdown().await
    };

    let server_to_client = async {
        io::copy(&mut ro, &mut wi).await?;
        wi.shutdown().await
    };

    // Wait for either of the threads to finish.
    tokio::try_join!(client_to_server, server_to_client)?;
    info!(target: "handshake_proxy::login", "Player \'{name}\' left the game");

    Ok(())
}
