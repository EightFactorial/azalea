use azalea_buf::{McBuf, UnsizedByteArray};
use azalea_protocol_macros::ClientboundForgePacket;

#[derive(Clone, Debug, McBuf, ClientboundForgePacket)]
pub struct ClientboundChannelMismatchDataPacket {
    pub data: UnsizedByteArray,
}
