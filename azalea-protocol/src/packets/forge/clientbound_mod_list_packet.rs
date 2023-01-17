use azalea_buf::McBuf;
use azalea_core::ResourceLocation;
use azalea_protocol_macros::ClientboundForgePacket;
use std::collections::HashMap;

#[derive(Clone, Debug, McBuf, ClientboundForgePacket)]
pub struct ClientboundModListPacket {
    pub mods: Vec<String>,
    pub channels: HashMap<ResourceLocation, String>,
    pub registries: Vec<ResourceLocation>,
    pub padding: u8,
}
