use azalea_buf::McBuf;
use azalea_protocol_macros::ServerboundForgePacket;

#[derive(Clone, Debug, Default, McBuf, ServerboundForgePacket)]
pub struct ServerboundAcknowledgePacket {}

impl ServerboundAcknowledgePacket {
    pub fn new() -> Self {
        ServerboundAcknowledgePacket {}
    }
}
