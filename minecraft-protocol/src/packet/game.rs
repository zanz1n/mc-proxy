use crate::{
    decoder::{Decoder, EnumDecoder},
    encoder::{Encoder, EnumEncoder},
    error::{DecodeError, EncodeError},
};
use minecraft_protocol_derive::{Decoder, Encoder};
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub enum GameServerBoundPacket {
    Other { type_id: u8 },
    ServerBoundPluginMessage(PlayPluginMessage),
}

#[derive(Debug, Clone)]
pub enum GameClientBoundPacket {
    Other { type_id: u8 },
    ClientBoundPluginMessage(PlayPluginMessage),
}

impl EnumEncoder for GameServerBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            GameServerBoundPacket::ServerBoundPluginMessage(_) => 0x10,
            GameServerBoundPacket::Other { type_id } => *type_id,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            GameServerBoundPacket::Other { type_id: _ } => Ok(()),
            GameServerBoundPacket::ServerBoundPluginMessage(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for GameServerBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x10 => {
                let plugin_message = PlayPluginMessage::decode(reader)?;

                Ok(GameServerBoundPacket::ServerBoundPluginMessage(
                    plugin_message,
                ))
            }
            type_id => Ok(GameServerBoundPacket::Other { type_id }),
        }
    }
}

impl EnumEncoder for GameClientBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            GameClientBoundPacket::Other { type_id } => *type_id,
            GameClientBoundPacket::ClientBoundPluginMessage(_) => 0x18,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            GameClientBoundPacket::Other { type_id: _ } => Ok(()),
            GameClientBoundPacket::ClientBoundPluginMessage(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for GameClientBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x18 => {
                let plugin_message = PlayPluginMessage::decode(reader)?;

                Ok(GameClientBoundPacket::ClientBoundPluginMessage(
                    plugin_message,
                ))
            }
            type_id => Ok(GameClientBoundPacket::Other { type_id }),
        }
    }
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct PlayPluginMessage {
    pub channel: String,
    #[data_type(with = "rest")]
    pub data: Vec<u8>,
}
