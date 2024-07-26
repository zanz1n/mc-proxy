use crate::decoder::{Decoder, EnumDecoder};
use crate::encoder::{Encoder, EnumEncoder};
use crate::error::{DecodeError, EncodeError};
use minecraft_protocol_derive::{Decoder, Encoder};
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub enum HandshakeServerBoundPacket {
    Handshake(Handshake),
}

impl EnumEncoder for HandshakeServerBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            HandshakeServerBoundPacket::Handshake(_) => 0x00,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            HandshakeServerBoundPacket::Handshake(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for HandshakeServerBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x00 => {
                let handshake = Handshake::decode(reader)?;
                Ok(HandshakeServerBoundPacket::Handshake(handshake))
            }
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct Handshake {
    #[data_type(with = "var_int")]
    pub protocol_version: i32,
    #[data_type(max_length = 255)]
    pub server_addr: String,
    pub server_port: u16,
    pub next_state: NextState,
}

#[derive(Encoder, Decoder, Debug, Clone)]
#[data_type(with = "var_int")]
pub enum NextState {
    Status = 1,
    Login = 2,
}
