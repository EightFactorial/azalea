pub mod client_intention_packet;

use std::fmt::Display;

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

impl Display for ClientIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ClientIdentifier::Forge => "\0FML3\0",
            _ => "",
        })
    }
}

impl ClientIdentifier {
    pub fn split_from_ip(ip: String) -> (String, ClientIdentifier) {
        let identifier = if ip.ends_with("\0FML3\0") {
            ClientIdentifier::Forge
        } else {
            ClientIdentifier::Vanilla
        };

        let hostname = match identifier {
            ClientIdentifier::Forge => ip
                .split_at(ip.len() - identifier.to_string().len())
                .0
                .to_string(),
            _ => ip,
        };

        (hostname, identifier)
    }
}
