use std::fmt::Display;

use azalea_protocol::packets::game::serverbound_chat_command_packet::ServerboundChatCommandPacket;

/// A command that can be run by clients connected to the proxy
///
/// These commands are invoked with `/proxy <command> [args...]`
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help(&'static str),
    Error(&'static str),
    About(&'static str),
    Disconnect(String),
    Echo(String),
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Help(_) => write!(f, "Help"),
            Command::Error(_) => write!(f, "Error"),
            Command::About(_) => write!(f, "About"),
            Command::Echo(_) => write!(f, "Echo"),
            Command::Disconnect(_) => write!(f, "Disconnect"),
        }
    }
}

/// An enum containing either a command or a command packet
#[derive(Debug, Clone)]
pub enum OptionalCommand {
    Some(Command),
    None(ServerboundChatCommandPacket),
}

impl OptionalCommand {
    /// Attempt to parse a proxy command from a chat command packet
    pub fn parse_packet(packet: ServerboundChatCommandPacket) -> OptionalCommand {
        let mut iter = packet.command.split_whitespace();

        if iter.next() != Some("proxy") {
            return OptionalCommand::None(packet);
        }

        OptionalCommand::Some(match iter.next() {
            // Echo a message back to the client
            Some("echo") => {
                let mut message = Vec::with_capacity(iter.size_hint().1.unwrap_or_default() + 1);
                message.push("[Proxy]");
                message.extend(iter);

                Command::Echo(message.join(" "))
            }
            // Disconnect from the proxy
            Some("disconnect") => {
                let reason = iter.collect::<Vec<&str>>().join(" ");
                Command::Disconnect(reason)
            }
            // Show the about message
            Some("about") => Command::About(ABOUT_MESSAGE),
            // Show a help message
            Some("help") | None => match iter.next() {
                // If no page is specified, show the first page
                None => Command::Help(HELP_MESSAGES[0]),
                // Otherwise, try to parse the page number
                Some(page) => match page.parse::<usize>() {
                    // If the page number is invalid, send an error message
                    Err(_) => Command::Error(INVALID_HELP_PAGE),
                    // Otherwise, show the specified page (or last page)
                    Ok(index) => {
                        Command::Help(HELP_MESSAGES[std::cmp::min(index, HELP_MESSAGES.len() - 1)])
                    }
                },
            },
            // If the command is invalid, send an error message
            Some(_) => Command::Error(UNKNOWN_COMMAND),
        })
    }
}

static UNKNOWN_COMMAND: &str = "Unknown proxy command. Type \"/proxy help\" for help.";

static INVALID_HELP_PAGE: &str = "Invalid help page. Type \"/proxy help\" for help.";
static HELP_MESSAGES: &[&str; 1] = &[
    r#"--- Showing proxy help page 1 of 1 (/proxy help <page>) ---
/proxy <help> <page> - Show this message
/proxy disconnect <reason> - Kick yourself from the proxy
/proxy echo <message> - Echo a message back to yourself
/proxy about - Show information about the proxy"#,
];

static ABOUT_MESSAGE: &str = r#"This proxy is powered by azalea-protocol,
a Minecraft protocol library written in Rust!
Check it out at https://github.com/azalea-rs/Azalea"#;
