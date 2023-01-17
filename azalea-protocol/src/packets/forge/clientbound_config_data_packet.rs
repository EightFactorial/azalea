use azalea_buf::McBuf;
use azalea_protocol_macros::ClientboundForgePacket;

#[derive(Clone, Debug, McBuf, ClientboundForgePacket)]
pub struct ClientboundConfigDataPacket {
    pub filename: String,
    pub filedata: Vec<u8>,
}
