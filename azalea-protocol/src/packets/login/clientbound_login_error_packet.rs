use azalea_buf::{McBuf, UnsizedByteArray};
use azalea_protocol_macros::ClientboundLoginPacket;

#[derive(Clone, Debug, McBuf, ClientboundLoginPacket)]
pub struct ClientboundLoginErrorPacket {
    pub stuff: UnsizedByteArray,
}
