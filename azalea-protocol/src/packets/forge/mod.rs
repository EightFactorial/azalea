pub mod clientbound_channel_mismatch_data_packet;
pub mod clientbound_config_data_packet;
pub mod clientbound_mod_data_packet;
pub mod clientbound_mod_list_packet;
pub mod clientbound_registry_packet;
pub mod serverbound_acknowledge_packet;
pub mod serverbound_mod_list_reply_packet;

use crate::read::{read_packet, ReadPacketError};
use crate::write::write_packet;
use azalea_protocol_macros::declare_state_packets;

use bytes::BytesMut;
use std::io::{Read, Write};
use tokio::io::{AsyncRead, AsyncWrite};

declare_state_packets!(
    ForgePacket,
    Serverbound => {
        0x02: serverbound_mod_list_reply_packet::ServerboundModListReplyPacket,
        0x63: serverbound_acknowledge_packet::ServerboundAcknowledgePacket,
    },
    Clientbound => {
        0x01: clientbound_mod_list_packet::ClientboundModListPacket,
        0x03: clientbound_registry_packet::ClientboundRegistryPacket,
        0x04: clientbound_config_data_packet::ClientboundConfigDataPacket,
        0x05: clientbound_mod_data_packet::ClientboundModDataPacket,
        0x06: clientbound_channel_mismatch_data_packet::ClientboundChannelMismatchDataPacket,
    }
);

impl ServerboundForgePacket {
    pub async fn write_to_buf(
        &self,
        buf: &mut (impl Write + AsyncWrite + Send + Unpin),
    ) -> Result<(), std::io::Error> {
        write_packet(self, buf, None, &mut None).await?;
        Ok(())
    }

    pub async fn write_to_vec(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut buf: Vec<u8> = Vec::new();
        self.write_to_buf(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_from_buf(
        buf: &mut (impl Read + AsyncRead + Send + Sync + Unpin),
    ) -> Result<ServerboundForgePacket, Box<ReadPacketError>> {
        Ok(
            read_packet::<ServerboundForgePacket, _>(buf, &mut BytesMut::new(), None, &mut None)
                .await?,
        )
    }
}

impl ClientboundForgePacket {
    pub async fn write_to_buf(
        &self,
        buf: &mut (impl Write + AsyncWrite + Send + Unpin),
    ) -> Result<(), std::io::Error> {
        write_packet(self, buf, None, &mut None).await?;
        Ok(())
    }

    pub async fn write_to_vec(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut buf: Vec<u8> = Vec::new();
        self.write_to_buf(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_from_buf(
        buf: &mut (impl Read + AsyncRead + Send + Sync + Unpin),
    ) -> Result<ClientboundForgePacket, Box<ReadPacketError>> {
        Ok(
            read_packet::<ClientboundForgePacket, _>(buf, &mut BytesMut::new(), None, &mut None)
                .await?,
        )
    }
}
