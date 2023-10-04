use azalea_protocol::{
    connect::Connection,
    packets::{
        configuration::{ClientboundConfigurationPacket, ServerboundConfigurationPacket},
        game::{ClientboundGamePacket, ServerboundGamePacket},
    },
    read::ReadPacketError,
};

/// A wrapper around a client connection that can
/// switch between the configuration and game states
pub enum ClientWrapper {
    Configuration(Connection<ServerboundConfigurationPacket, ClientboundConfigurationPacket>),
    Game(Connection<ServerboundGamePacket, ClientboundGamePacket>),
}

/// An enum containing the two types of client packets
#[derive(Debug)]
pub enum ClientWrapperPacket {
    Configuration(ServerboundConfigurationPacket),
    Game(ServerboundGamePacket),
}

impl ClientWrapper {
    /// Switch to the game state
    pub fn game(self) -> Self {
        match self {
            ClientWrapper::Configuration(conn) => ClientWrapper::Game(Connection::from(conn)),
            conn => conn,
        }
    }

    /// Switch to the configuration state
    pub fn configuration(self) -> Self {
        match self {
            ClientWrapper::Game(conn) => ClientWrapper::Configuration(Connection::from(conn)),
            conn => conn,
        }
    }

    /// Read a packet from the client
    pub async fn read(&mut self) -> Result<ClientWrapperPacket, Box<ReadPacketError>> {
        match self {
            ClientWrapper::Configuration(conn) => {
                Ok(ClientWrapperPacket::Configuration(conn.read().await?))
            }
            ClientWrapper::Game(conn) => Ok(ClientWrapperPacket::Game(conn.read().await?)),
        }
    }

    /// Write a packet to the client
    pub async fn write(&mut self, packet: TargetWrapperPacket) -> std::io::Result<()> {
        match self {
            ClientWrapper::Configuration(conn) => match packet {
                TargetWrapperPacket::Configuration(packet) => {
                    conn.write(packet).await?;
                }
                TargetWrapperPacket::Game(_) => {
                    panic!("Attempted to write a game packet to a configuration connection")
                }
            },
            ClientWrapper::Game(conn) => match packet {
                TargetWrapperPacket::Configuration(_) => {
                    panic!("Attempted to write a configuration packet to a game connection")
                }
                TargetWrapperPacket::Game(packet) => {
                    conn.write(packet).await?;
                }
            },
        }

        Ok(())
    }
}

/// A wrapper around a target connection that can
/// switch between the configuration and game states
pub enum TargetWrapper {
    Configuration(Connection<ClientboundConfigurationPacket, ServerboundConfigurationPacket>),
    Game(Connection<ClientboundGamePacket, ServerboundGamePacket>),
}

/// An enum containing the two types of target packets
#[derive(Debug)]
pub enum TargetWrapperPacket {
    Configuration(ClientboundConfigurationPacket),
    Game(ClientboundGamePacket),
}

impl TargetWrapper {
    /// Switch to the game state
    pub fn game(self) -> Self {
        match self {
            TargetWrapper::Configuration(conn) => TargetWrapper::Game(Connection::from(conn)),
            conn => conn,
        }
    }

    /// Switch to the configuration state
    pub fn configuration(self) -> Self {
        match self {
            TargetWrapper::Game(conn) => TargetWrapper::Configuration(Connection::from(conn)),
            conn => conn,
        }
    }

    /// Read a packet from the target server
    pub async fn read(&mut self) -> Result<TargetWrapperPacket, Box<ReadPacketError>> {
        match self {
            TargetWrapper::Configuration(conn) => {
                Ok(TargetWrapperPacket::Configuration(conn.read().await?))
            }
            TargetWrapper::Game(conn) => Ok(TargetWrapperPacket::Game(conn.read().await?)),
        }
    }

    /// Write a packet to the target server
    pub async fn write(&mut self, packet: ClientWrapperPacket) -> std::io::Result<()> {
        match self {
            TargetWrapper::Configuration(conn) => match packet {
                ClientWrapperPacket::Configuration(packet) => {
                    conn.write(packet).await?;
                }
                ClientWrapperPacket::Game(_) => {
                    panic!("Attempted to write a game packet to a configuration connection")
                }
            },
            TargetWrapper::Game(conn) => match packet {
                ClientWrapperPacket::Configuration(_) => {
                    panic!("Attempted to write a configuration packet to a game connection")
                }
                ClientWrapperPacket::Game(packet) => {
                    conn.write(packet).await?;
                }
            },
        }

        Ok(())
    }
}
