use azalea_buf::{BufReadError, McBufReadable, McBufVarReadable, McBufVarWritable, McBufWritable};
use azalea_chat::Component;
use azalea_core::ResourceLocation;
use azalea_protocol_macros::ClientboundStatusPacket;
use serde::{de::Visitor, Deserialize, Serialize};
use serde_json::{value::Serializer, Value};
use std::{
    collections::HashMap,
    io::{Cursor, Write},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SamplePlayer {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Players {
    pub max: i32,
    pub online: i32,
    #[serde(default)]
    pub sample: Vec<SamplePlayer>,
}

// the entire packet is just json, which is why it has deserialize
#[derive(Clone, Debug, Serialize, Deserialize, ClientboundStatusPacket)]
pub struct ClientboundStatusResponsePacket {
    pub description: Component,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    pub players: Players,
    pub version: Version,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "previewsChat")]
    pub previews_chat: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "forgeData")]
    pub fml: Option<ForgeData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForgeData {
    pub mods: Vec<String>,
    pub channels: Vec<String>,
    pub truncated: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "d")]
    pub encoded: Option<ForgeDataEncoded>,
}

// Essentially 15 bits of binary data are encoded into every UTF-16 code point.
// The resulting string is then stored in the "d" property of the resulting
// JSON.
#[derive(Clone, Debug)]
pub struct ForgeDataEncoded {
    pub mods: HashMap<String, String>,
    pub channels: HashMap<ResourceLocation, (String, bool)>,
    pub truncated: bool,
}

impl Serialize for ForgeDataEncoded {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut buf: Vec<u8> = Vec::new();

        // Write all data to buffer
        self.truncated
            .write_into(&mut buf)
            .expect("Error writing to buffer");
        (self.mods.len() as u16)
            .write_into(&mut buf)
            .expect("Error writing to buffer");
        for (mod_id, mod_version) in self.mods.clone() {
            let channel_info = self
                .channels
                .iter()
                .filter(|(channel_name, _)| channel_name.namespace == *mod_id)
                .map(|(channel_name, (version, required))| {
                    (channel_name.namespace.clone(), version.clone(), *required)
                })
                .collect::<Vec<_>>();

            let channel_size = channel_info.len();
            let flag = (channel_size << 1) | if &mod_version == "None" { 0b1 } else { 0b0 };
            (flag as u32)
                .var_write_into(&mut buf)
                .expect("Error writing to buffer");
            mod_id
                .write_into(&mut buf)
                .expect("Error writing to buffer");

            if &mod_version != "None" {
                mod_version
                    .write_into(&mut buf)
                    .expect("Error writing to buffer");
            }

            for (channel_name, version, required) in channel_info {
                channel_name
                    .write_into(&mut buf)
                    .expect("Error writing to buffer");
                version
                    .write_into(&mut buf)
                    .expect("Error writing to buffer");
                required
                    .write_into(&mut buf)
                    .expect("Error writing to buffer");
            }

            let non_mod_channels = self
                .channels
                .iter()
                .filter(|(channel_name, _)| !self.mods.contains_key(&channel_name.namespace))
                .map(|(channel_name, (version, required))| {
                    (channel_name.clone(), version.clone(), *required)
                })
                .collect::<Vec<_>>();

            (non_mod_channels.len() as u32)
                .var_write_into(&mut buf)
                .expect("Error writing to buffer");
            for (channel_name, version, required) in non_mod_channels {
                channel_name
                    .write_into(&mut buf)
                    .expect("Error writing to buffer");
                version
                    .write_into(&mut buf)
                    .expect("Error writing to buffer");
                required
                    .write_into(&mut buf)
                    .expect("Error writing to buffer");
            }
        }

        // Encode buffer to custom format
        let byte_length = buf.len();
        let mut result = String::new();
        result.push(char::from_u32((byte_length & 0x7FFF) as u32).unwrap());
        result.push(char::from_u32(((byte_length >> 15) & 0x7FFF) as u32).unwrap());

        let mut buffer: u32 = 0;
        let mut bits_in_buf = 0;
        for b in buf {
            if bits_in_buf >= 15 {
                let c = (buffer & 0x7FFF) as u16;
                result.push(char::from_u32(c as u32).unwrap());
                buffer >>= 15;
                bits_in_buf -= 15;
            }
            buffer |= (b as u32) << bits_in_buf;
            bits_in_buf += 8;
        }

        if bits_in_buf > 0 {
            let c = (buffer & 0x7FFF) as u16;
            result.push(char::from_u32(c as u32).unwrap());
        }

        serializer.serialize_str(result.as_str())
    }
}

impl<'de> Deserialize<'de> for ForgeDataEncoded {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ForgeDataEncodedVisitor;
        impl<'de> Visitor<'de> for ForgeDataEncodedVisitor {
            type Value = ForgeDataEncoded;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid String")
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Decode bytes from custom format
                let mut decoded = vec![];
                {
                    let mut buffer: u32 = 0;
                    let mut bits_in_buf = 0;
                    for c in v.chars().skip(2) {
                        buffer |= (c as u32) << bits_in_buf;
                        bits_in_buf += 15;
                        while bits_in_buf >= 8 {
                            decoded.push((buffer & 0xff) as u8);
                            buffer >>= 8;
                            bits_in_buf -= 8;
                        }
                    }
                }

                // Now read bytes
                let mut mods = HashMap::new();
                let mut channels = HashMap::new();

                let mut buf: Cursor<&[u8]> = Cursor::new(&decoded);
                let truncated = bool::read_from(&mut buf).expect("Error reading truncated");

                for _ in 0..u16::read_from(&mut buf).expect("Error reading mod length") {
                    let flag = u32::var_read_from(&mut buf).expect("Error reading mod flag");
                    let channel_size = flag >> 1;
                    let is_ignore_server_only = (flag & 0b1) != 0;

                    let mod_id = String::read_from(&mut buf).expect("Error reading mod id");
                    let mod_version = if is_ignore_server_only {
                        "None".to_string()
                    } else {
                        String::read_from(&mut buf).expect("Error reading mod version")
                    };

                    for _ in 0..channel_size {
                        let channel_name =
                            String::read_from(&mut buf).expect("Error reading channel name");
                        let channel_ver =
                            String::read_from(&mut buf).expect("Error reading channel version");
                        let required = bool::read_from(&mut buf)
                            .expect("Error reading channel requried status");
                        channels.insert(
                            ResourceLocation::new(format!("{mod_id}:{channel_name}").as_str())
                                .expect("Invalid ResourceLocation from mod id and channel name"),
                            (channel_ver, required),
                        );
                    }

                    mods.insert(mod_id, mod_version);
                }

                let non_mod_channels =
                    u32::var_read_from(&mut buf).expect("Error reading non mod channel length");
                for _ in 0..non_mod_channels {
                    let channel_name =
                        ResourceLocation::read_from(&mut buf).expect("Error reading channel name");
                    let channel_ver =
                        String::read_from(&mut buf).expect("Error reading channel version");
                    let required =
                        bool::read_from(&mut buf).expect("Error reading channel required status");
                    channels.insert(channel_name, (channel_ver, required));
                }

                Ok(ForgeDataEncoded {
                    mods,
                    channels,
                    truncated,
                })
            }
        }
        deserializer.deserialize_string(ForgeDataEncodedVisitor)
    }
}

impl McBufReadable for ClientboundStatusResponsePacket {
    fn read_from(buf: &mut Cursor<&[u8]>) -> Result<ClientboundStatusResponsePacket, BufReadError> {
        let status_string = String::read_from(buf)?;
        let status_json: Value = serde_json::from_str(status_string.as_str())?;

        Ok(ClientboundStatusResponsePacket::deserialize(status_json)?)
    }
}

impl McBufWritable for ClientboundStatusResponsePacket {
    fn write_into(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        let status_string = ClientboundStatusResponsePacket::serialize(self, Serializer)
            .unwrap()
            .to_string();
        status_string.write_into(buf)?;
        Ok(())
    }
}
