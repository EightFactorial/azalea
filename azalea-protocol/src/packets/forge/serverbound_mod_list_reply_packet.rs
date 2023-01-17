use azalea_buf::{McBufReadable, McBufWritable};
use azalea_core::ResourceLocation;
use azalea_protocol_macros::ServerboundForgePacket;
use std::collections::HashMap;

use super::clientbound_mod_list_packet::ClientboundModListPacket;

#[derive(Clone, Debug, ServerboundForgePacket)]
pub struct ServerboundModListReplyPacket {
    pub mods: Vec<String>,
    pub channels: HashMap<ResourceLocation, String>,
    pub registries: HashMap<ResourceLocation, String>,
}

impl McBufWritable for ServerboundModListReplyPacket {
    fn write_into(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        self.mods.write_into(buf)?;
        self.channels.write_into(buf)?;
        self.registries.write_into(buf)?;
        Ok(())
    }
}

impl McBufReadable for ServerboundModListReplyPacket {
    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, azalea_buf::BufReadError> {
        Ok(Self {
            mods: Vec::<String>::read_from(buf)?,
            channels: HashMap::<ResourceLocation, String>::read_from(buf)?,
            registries: HashMap::<ResourceLocation, String>::read_from(buf)?,
        })
    }
}

impl From<ClientboundModListPacket> for ServerboundModListReplyPacket {
    fn from(value: ClientboundModListPacket) -> Self {
        let mut registries: HashMap<ResourceLocation, String> = HashMap::new();
        value.registries.iter().for_each(|x| {
            registries.insert(x.to_owned(), "".to_string());
        });
        Self {
            mods: value.mods,
            channels: value.channels,
            registries,
        }
    }
}
