use azalea_buf::{McBuf, McBufReadable, McBufWritable, UnsizedByteArray};
use azalea_core::ResourceLocation;
use azalea_protocol_macros::ServerboundLoginPacket;

#[derive(Clone, Debug, McBuf, ServerboundLoginPacket)]
pub struct ServerboundCustomQueryPacket {
    #[var]
    pub transaction_id: u32,
    pub query: Option<CustomQuery>,
}

#[derive(Clone, Debug)]
pub struct CustomQuery {
    pub identifier: Option<ResourceLocation>,
    pub data: UnsizedByteArray,
}

impl McBufReadable for CustomQuery {
    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, azalea_buf::BufReadError> {
        let pos = buf.position();
        if let Ok(identifier) = ResourceLocation::read_from(buf) {
            Ok(Self {
                identifier: Some(identifier),
                data: UnsizedByteArray::read_from(buf)?,
            })
        } else {
            buf.set_position(pos);
            Ok(Self {
                identifier: None,
                data: UnsizedByteArray::read_from(buf)?,
            })
        }
    }
}

impl McBufWritable for CustomQuery {
    fn write_into(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        if let Some(identifier) = &self.identifier {
            identifier.write_into(buf)?;
        }
        self.data.write_into(buf)?;
        Ok(())
    }
}
