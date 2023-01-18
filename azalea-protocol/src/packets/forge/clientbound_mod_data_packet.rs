use std::collections::HashMap;

use azalea_buf::{McBufReadable, McBufVarReadable, McBufVarWritable, McBufWritable};
use azalea_protocol_macros::ClientboundForgePacket;

#[derive(Clone, Debug, ClientboundForgePacket)]
pub struct ClientboundModDataPacket {
    pub list: HashMap<String, (String, String)>,
}

impl McBufReadable for ClientboundModDataPacket {
    fn read_from(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, azalea_buf::BufReadError> {
        let mut list: HashMap<String, (String, String)> = HashMap::new();
        for _ in 0..u32::var_read_from(buf)? {
            list.insert(
                String::read_from(buf)?,
                (String::read_from(buf)?, String::read_from(buf)?),
            );
        }
        Ok(Self { list })
    }
}

impl McBufWritable for ClientboundModDataPacket {
    fn write_into(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        (self.list.len() as u32).var_write_into(buf)?;
        for (id, (name, version)) in &self.list {
            id.write_into(buf)?;
            name.write_into(buf)?;
            version.write_into(buf)?;
        }
        Ok(())
    }
}
