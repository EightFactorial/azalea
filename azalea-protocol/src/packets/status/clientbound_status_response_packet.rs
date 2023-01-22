use azalea_buf::{BufReadError, McBufReadable, McBufVarReadable, McBufVarWritable, McBufWritable};
use azalea_chat::Component;
use azalea_protocol_macros::ClientboundStatusPacket;
use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};
use serde_json::{value::Serializer, Value};
use std::{
    collections::HashMap,
    io::{Cursor, Write},
};

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
    pub forge: Option<ForgeData>,
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

// Essentially 15 bits of binary data are encoded into every UTF-16 code point.
// The resulting string is then stored in the "d" property of the resulting
// JSON. The functions impl'd handle this conversion, and the rest is
// handling this transparently for the user.
#[derive(Clone, Debug)]
pub struct ForgeData {
    pub mods: Vec<ForgeModData>,
    pub channels: Vec<ForgeChannelData>,
    pub truncated: bool,
    pub fmlversion: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForgeModData {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForgeChannelData {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

impl Serialize for ForgeData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.truncated {
            match self.encode_optimized() {
                Err(e) => Err(serde::ser::Error::custom(e)),
                Ok(s) => {
                    let mut ser = serializer.serialize_struct("", 5)?;
                    ser.serialize_field("mods", &HashMap::<String, Option<String>>::new())?;
                    ser.serialize_field("channels", &Vec::<ForgeChannelData>::new())?;
                    ser.serialize_field("truncated", &self.truncated)?;
                    ser.serialize_field("d", &s)?;
                    ser.serialize_field("fmlNetworkVersion", &self.fmlversion)?;
                    ser.end()
                }
            }
        } else {
            let mut ser = serializer.serialize_struct("", 4)?;
            ser.serialize_field("mods", &self.mods)?;
            ser.serialize_field("channels", &self.channels)?;
            ser.serialize_field("truncated", &self.truncated)?;
            ser.serialize_field("fmlNetworkVersion", &self.fmlversion)?;
            ser.end()
        }
    }
}

impl<'de> Deserialize<'de> for ForgeData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Mods,
            Channels,
            Truncated,
            D,
            #[serde(rename = "fmlNetworkVersion")]
            FMLNetworkVersion,
        }

        struct ForgeDataVisitor;
        impl<'de> Visitor<'de> for ForgeDataVisitor {
            type Value = ForgeData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct ForgeData")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mods = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let channels = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                let truncated = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
                let fmlversion = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

                let d = match seq.next_element() {
                    Err(_) => None,
                    Ok(x) => x,
                };

                if truncated {
                    if let Some(data) = d {
                        let data = ForgeData::deserialize_optimized(
                            ForgeData::decode_optimized(data),
                            fmlversion,
                        );
                        match data {
                            Ok(data) => return Ok(data),
                            Err(e) => return Err(serde::de::Error::custom(e)),
                        }
                    } else {
                        return Err(serde::de::Error::custom(
                            "Packet is labled truncated but no data provided",
                        ));
                    }
                }

                Ok(ForgeData {
                    mods,
                    channels,
                    truncated,
                    fmlversion,
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut mods = None;
                let mut channels = None;
                let mut truncated = None;
                let mut fmlversion = None;
                let mut d = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Mods => {
                            if mods.is_some() {
                                return Err(serde::de::Error::duplicate_field("mods"));
                            }
                            mods = Some(map.next_value()?);
                        }
                        Field::Channels => {
                            if channels.is_some() {
                                return Err(serde::de::Error::duplicate_field("channels"));
                            }
                            channels = Some(map.next_value()?);
                        }
                        Field::Truncated => {
                            if truncated.is_some() {
                                return Err(serde::de::Error::duplicate_field("truncated"));
                            }
                            truncated = Some(map.next_value()?);
                        }
                        Field::FMLNetworkVersion => {
                            if fmlversion.is_some() {
                                return Err(serde::de::Error::duplicate_field("fmlversion"));
                            }
                            fmlversion = Some(map.next_value()?);
                        }
                        Field::D => {
                            if d.is_some() {
                                return Err(serde::de::Error::duplicate_field("d"));
                            }
                            d = Some(map.next_value()?);
                        }
                    }
                }

                let mods = mods.ok_or_else(|| serde::de::Error::missing_field("mods"))?;
                let channels =
                    channels.ok_or_else(|| serde::de::Error::missing_field("channels"))?;
                let truncated =
                    truncated.ok_or_else(|| serde::de::Error::missing_field("truncated"))?;
                let fmlversion =
                    fmlversion.ok_or_else(|| serde::de::Error::missing_field("fmlversion"))?;

                if truncated {
                    if let Some(data) = d {
                        let data = ForgeData::deserialize_optimized(
                            ForgeData::decode_optimized(data),
                            fmlversion,
                        );
                        match data {
                            Ok(data) => return Ok(data),
                            Err(e) => return Err(serde::de::Error::custom(e)),
                        }
                    } else {
                        return Err(serde::de::Error::custom(
                            "Packet is labled truncated but no data provided",
                        ));
                    }
                }

                Ok(ForgeData {
                    mods,
                    channels,
                    truncated,
                    fmlversion,
                })
            }
        }

        const FIELDS: &[&str] = &["mods", "channels", "truncated", "d", "fmlNetworkVersion"];
        deserializer.deserialize_struct("", FIELDS, ForgeDataVisitor)
    }
}

impl ForgeData {
    fn encode_optimized(&self) -> Result<String, std::io::Error> {
        let mut buf = Vec::new();
        self.serialize_optimized(&mut buf)?;

        let mut result = String::new();
        let byte_length = buf.len();
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

        Ok(result)
    }

    fn decode_optimized(value: String) -> Vec<u8> {
        let mut decoded = vec![];
        {
            let mut buffer: u32 = 0;
            let mut bits_in_buf = 0;
            for c in value.chars().skip(2) {
                buffer |= (c as u32) << bits_in_buf;
                bits_in_buf += 15;
                while bits_in_buf >= 8 {
                    decoded.push((buffer & 0xff) as u8);
                    buffer >>= 8;
                    bits_in_buf -= 8;
                }
            }
        }
        decoded
    }

    fn serialize_optimized(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        self.truncated.write_into(buf)?;
        (self.mods.len() as u16).write_into(buf)?;
        for mod_data in self.mods.clone() {
            let channel_info = self
                .channels
                .iter()
                .filter(|x| x.name == *mod_data.name)
                .map(|x| (x.name.clone(), x.version.clone(), x.optional))
                .collect::<Vec<_>>();

            let channel_size = channel_info.len();
            let flag = (channel_size << 1) | if mod_data.version.is_some() { 0b1 } else { 0b0 };
            (flag as u32).var_write_into(buf)?;
            mod_data.name.write_into(buf)?;

            if let Some(version) = mod_data.version {
                version.write_into(buf)?;
            }

            for (channel_name, version, optional) in channel_info {
                channel_name.write_into(buf)?;
                version.write_into(buf)?;
                optional.write_into(buf)?;
            }

            let non_mod_channels = self
                .channels
                .iter()
                .filter(|x| self.mods.iter().filter(|m| m.name == x.name).count() == 0)
                .map(|x| (x.name.clone(), x.version.clone(), x.optional))
                .collect::<Vec<_>>();

            (non_mod_channels.len() as u32).var_write_into(buf)?;
            for (channel_name, version, optional) in non_mod_channels {
                channel_name.write_into(buf)?;
                version.write_into(buf)?;
                optional.write_into(buf)?;
            }
        }
        Ok(())
    }

    fn deserialize_optimized(value: Vec<u8>, fmlversion: u8) -> Result<ForgeData, BufReadError> {
        let mut mods = Vec::new();
        let mut channels = Vec::new();

        let mut buf: Cursor<&[u8]> = Cursor::new(&value);

        // This is the 'not truncated' version of the data
        // inside the packet, but we don't care, the packet
        // outside has been truncated, so it's always true.
        let _ = bool::read_from(&mut buf)?;
        let truncated = true;

        for _ in 0..u16::read_from(&mut buf)? {
            let flag = u32::var_read_from(&mut buf)?;
            let channel_size = flag >> 1;
            let is_ignore_server_only = (flag & 0b1) != 0;

            let mod_id = String::read_from(&mut buf)?;
            let mod_version = if is_ignore_server_only {
                None
            } else {
                Some(String::read_from(&mut buf)?)
            };

            for _ in 0..channel_size {
                let channel_name = String::read_from(&mut buf)?;
                let channel_ver = String::read_from(&mut buf)?;
                let optional = bool::read_from(&mut buf)?;

                channels.push(ForgeChannelData {
                    name: format!("{mod_id}:{channel_name}"),
                    version: channel_ver,
                    optional,
                })
            }

            mods.push(ForgeModData {
                name: mod_id,
                version: mod_version,
            });
        }

        let non_mod_channels = u32::var_read_from(&mut buf)?;
        for _ in 0..non_mod_channels {
            let channel_name = String::read_from(&mut buf)?;
            let channel_ver = String::read_from(&mut buf)?;
            let optional = bool::read_from(&mut buf)?;

            channels.push(ForgeChannelData {
                name: channel_name,
                version: channel_ver,
                optional,
            });
        }

        Ok(Self {
            mods,
            channels,
            truncated,
            fmlversion,
        })
    }
}
