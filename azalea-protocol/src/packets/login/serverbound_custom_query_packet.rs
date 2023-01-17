use azalea_buf::{McBuf, UnsizedByteArray};
use azalea_core::ResourceLocation;
use azalea_protocol_macros::ServerboundLoginPacket;

#[derive(Clone, Debug, McBuf, ServerboundLoginPacket)]
pub struct ServerboundCustomQueryPacket {
    #[var]
    pub transaction_id: u32,
    pub query: Option<CustomQuery>,
}

#[derive(Clone, Debug, McBuf)]
pub struct CustomQuery {
    pub identifier: ResourceLocation,
    pub data: UnsizedByteArray,
}
