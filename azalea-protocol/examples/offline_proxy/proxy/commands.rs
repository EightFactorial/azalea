use azalea_protocol::packets::game::serverbound_chat_command_packet::ServerboundChatCommandPacket;

/// An enum containing either a command or a packet
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

/// Optionally parse a proxy command from a chat command packet
pub fn parse_command(packet: ServerboundChatCommandPacket) -> anyhow::Result<OptionalCommand> {
    let mut iter = packet.command.split_whitespace();

    match iter.next() {
        Some("proxy::disconnect") => {
            let reason = iter.collect::<Vec<&str>>().join(" ");
            Ok(OptionalCommand::Some(Command::Disconnect(reason)))
        }
        Some("proxy::echo") => {
            let message = iter.collect::<Vec<&str>>().join(" ");
            Ok(OptionalCommand::Some(Command::Echo(message)))
        }
        None | Some(_) => Ok(OptionalCommand::None(packet)),
    }
}
