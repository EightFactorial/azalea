use azalea_buf::McBuf;
use azalea_core::ResourceLocation;

#[derive(Debug, Clone, McBuf)]
pub struct ClientboundFabricRegistryPacket {
    pub registry: Vec<ResourceLocation>,
}
