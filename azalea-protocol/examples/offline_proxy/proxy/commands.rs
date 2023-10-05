use azalea_protocol::packets::game::serverbound_chat_command_packet::ServerboundChatCommandPacket;

/// An enum containing either a command or a command packet
#[derive(Debug, Clone)]
pub enum OptionalCommand {
    Some(Command),
    None(ServerboundChatCommandPacket),
}

/// A command that can be run by clients connected to the proxy
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Disconnect(String),
    Echo(String),
}

impl OptionalCommand {
    /// Attempt to parse a proxy command from a chat command packet
    pub fn parse_packet(packet: ServerboundChatCommandPacket) -> OptionalCommand {
        let mut iter = packet.command.split_whitespace();

        match iter.next() {
            Some("proxy::disconnect") => {
                let reason = iter.collect::<Vec<&str>>().join(" ");
                OptionalCommand::Some(Command::Disconnect(reason))
            }
            Some("proxy::echo") => {
                let message = iter.collect::<Vec<&str>>().join(" ");
                OptionalCommand::Some(Command::Echo(message))
            }
            None | Some(_) => OptionalCommand::None(packet),
        }
    }
}
