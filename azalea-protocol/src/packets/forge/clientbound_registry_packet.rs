use azalea_buf::McBuf;
use azalea_core::ResourceLocation;
use azalea_protocol_macros::ClientboundForgePacket;
use std::collections::HashMap;

#[derive(Clone, Debug, McBuf, ClientboundForgePacket)]
pub struct ClientboundRegistryPacket {
    pub registry: ResourceLocation,
    pub data: Option<ForgeRegistryData>,
}

#[derive(Clone, Debug, McBuf)]
pub struct ForgeRegistryData {
    #[var]
    pub ids: HashMap<ResourceLocation, u32>,
    pub aliases: HashMap<ResourceLocation, ResourceLocation>,
    pub overrides: HashMap<ResourceLocation, String>,
    #[var]
    pub blocked: Vec<u32>,
    pub dummied: Vec<ResourceLocation>,
}
