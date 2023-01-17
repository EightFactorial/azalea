use crate::packets::ConnectionProtocol;
use azalea_buf::{McBufReadable, McBufVarWritable, McBufWritable};
use azalea_protocol_macros::ServerboundHandshakePacket;
use std::hash::Hash;

pub use super::ClientIdentifier;

#[derive(Hash, Clone, Debug, ServerboundHandshakePacket)]
pub struct ClientIntentionPacket {
    #[var]
    pub protocol_version: u32,
    pub hostname: String,
    pub port: u16,
    pub intention: ConnectionProtocol,
    pub identifier: ClientIdentifier,
}

impl McBufWritable for ClientIntentionPacket {
    fn write_into(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        self.protocol_version.var_write_into(buf)?;
        format!("{0}{1}", self.hostname, self.identifier.to_string()).write_into(buf)?;
        self.port.write_into(buf)?;
        self.intention.write_into(buf)?;
        Ok(())
    }
}

impl McBufReadable for ClientIntentionPacket {
    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, azalea_buf::BufReadError> {
        let protocol_version = u32::read_from(buf)?;
        let (hostname, identifier) = ClientIdentifier::split_from_ip(String::read_from(buf)?);
        Ok(Self {
            protocol_version,
            hostname,
            identifier,
            port: u16::read_from(buf)?,
            intention: ConnectionProtocol::read_from(buf)?,
        })
    }
}
