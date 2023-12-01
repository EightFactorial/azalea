use azalea_protocol::{
    connect::Connection,
    packets::{
        handshaking::{
            client_intention_packet::ClientIntentionPacket, ClientboundHandshakePacket,
            ServerboundHandshakePacket,
        },
        login::serverbound_hello_packet::ServerboundHelloPacket,
    },
};
use std::{error::Error, net::SocketAddr};
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;

/// Create a connection to the proxy target,
/// repeat the packets recieved earlier, and
/// forward data from the connection to the proxy target.
pub async fn proxy(
    mut inbound: TcpStream,
    target_addr: SocketAddr,
    intent: ClientIntentionPacket,
    hello: ServerboundHelloPacket,
) -> Result<(), Box<dyn Error>> {
    let outbound = TcpStream::connect(target_addr).await?;
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
    info!(target: "handshake_proxy::login", "Player `{name}` left the game");

    Ok(())
}
