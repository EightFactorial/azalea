pub mod client_intention_packet;

use azalea_protocol_macros::declare_state_packets;

declare_state_packets!(
    HandshakePacket,
    Serverbound => {
        0x00: client_intention_packet::ClientIntentionPacket,
    },
    Clientbound => {}
);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]

pub enum ClientIdentifier {
    Vanilla,
    Forge,
    Fabric,
}

impl Default for ClientIdentifier {
    fn default() -> Self {
        Self::Vanilla
    }
}

impl ClientIdentifier {
    pub fn to_string(&self) -> String {
        match &self {
            ClientIdentifier::Vanilla => String::new(),
            ClientIdentifier::Forge => String::from("\0FML3\0"),
            ClientIdentifier::Fabric => todo!(),
        }
    }

    pub fn split_from_ip(ip: String) -> (String, ClientIdentifier) {
        let identifier = if ip.ends_with("\0FML3\0") {
            ClientIdentifier::Forge
        } else {
            ClientIdentifier::Vanilla
        };
        let hostname = match identifier {
            ClientIdentifier::Vanilla => ip,
            _ => ip
                .split_at(ip.len() - identifier.to_string().len())
                .0
                .to_string(),
        };
        (hostname, identifier)
    }
}
