use azalea_buf::{BufReadError, McBufReadable, McBufWritable};
use azalea_chat::Component;
use azalea_protocol_macros::ClientboundStatusPacket;
use serde::{de::Visitor, Deserialize, Serialize};
use serde_json::{value::Serializer, Value};
use std::io::{Cursor, Write};

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
    pub data: Option<ForgePingData>,
}

// Essentially 15 bits of binary data are encoded into every UTF-16 code point.
// The resulting string is then stored in the "d" property of the resulting
// JSON.
#[derive(Clone, Debug)]
pub struct ForgePingData {
    pub data: Vec<u8>,
}

impl Serialize for ForgePingData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Ok(string) = String::from_utf8(self.data.clone()) {
            serializer.serialize_str(&string)
        } else {
            Err(serde::ser::Error::custom("Inavlid String"))
        }
    }
}

impl<'de> Deserialize<'de> for ForgePingData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ForgePingDataVisitor;
        impl<'de> Visitor<'de> for ForgePingDataVisitor {
            type Value = ForgePingData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid String")
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ForgePingData {
                    data: v.as_bytes().to_vec(),
                })
            }
        }
        deserializer.deserialize_string(ForgePingDataVisitor)
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
