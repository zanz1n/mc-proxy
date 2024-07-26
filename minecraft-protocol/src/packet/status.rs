use crate::decoder::Decoder;
use crate::encoder::{Encoder, EnumEncoder};
use crate::error::{DecodeError, EncodeError};
use crate::{data::server_status::*, decoder::EnumDecoder};
use minecraft_protocol_derive::{Decoder, Encoder};
use std::io::{Read, Write};

#[derive(Debug, Clone)]
pub enum StatusServerBoundPacket {
    StatusRequest,
    PingRequest(PingRequest),
}

#[derive(Debug, Clone)]
pub enum StatusClientBoundPacket {
    StatusResponse(StatusResponse),
    PingResponse(PingResponse),
}

impl EnumEncoder for StatusServerBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            StatusServerBoundPacket::StatusRequest => 0x00,
            StatusServerBoundPacket::PingRequest(_) => 0x01,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            StatusServerBoundPacket::StatusRequest => Ok(()),
            StatusServerBoundPacket::PingRequest(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for StatusServerBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x00 => Ok(StatusServerBoundPacket::StatusRequest),
            0x01 => {
                let ping_request = PingRequest::decode(reader)?;

                Ok(StatusServerBoundPacket::PingRequest(ping_request))
            }
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

impl EnumEncoder for StatusClientBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            StatusClientBoundPacket::StatusResponse(_) => 0x00,
            StatusClientBoundPacket::PingResponse(_) => 0x01,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            StatusClientBoundPacket::StatusResponse(packet) => packet.encode(writer),
            StatusClientBoundPacket::PingResponse(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for StatusClientBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x00 => {
                let status_response = StatusResponse::decode(reader)?;

                Ok(StatusClientBoundPacket::StatusResponse(status_response))
            }
            0x01 => {
                let ping_reponse = PingResponse::decode(reader)?;

                Ok(StatusClientBoundPacket::PingResponse(ping_reponse))
            }
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct PingRequest {
    pub time: u64,
}

impl PingRequest {
    pub fn new(time: u64) -> StatusServerBoundPacket {
        let ping_request = PingRequest { time };

        StatusServerBoundPacket::PingRequest(ping_request)
    }
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct PingResponse {
    pub time: u64,
}

impl PingResponse {
    pub fn new(time: u64) -> StatusClientBoundPacket {
        let ping_response = PingResponse { time };

        StatusClientBoundPacket::PingResponse(ping_response)
    }
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct StatusResponse {
    pub server_status: ServerStatus,
}

impl StatusResponse {
    pub fn new(server_status: ServerStatus) -> StatusClientBoundPacket {
        let status_response = StatusResponse { server_status };

        StatusClientBoundPacket::StatusResponse(status_response)
    }
}

#[cfg(test)]
mod tests {
    use crate::data::chat::{Message, Payload};
    use crate::decoder::Decoder;
    use crate::encoder::Encoder;
    use crate::packet::status::*;
    use std::io::Cursor;
    use uuid::Uuid;

    #[test]
    fn test_ping_request_encode() {
        let ping_request = PingRequest {
            time: 1577735845610,
        };

        let mut vec = Vec::new();
        ping_request.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/status/ping_request.dat").to_vec()
        );
    }

    #[test]
    fn test_status_ping_request_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/status/ping_request.dat").to_vec());
        let ping_request = PingRequest::decode(&mut cursor).unwrap();

        assert_eq!(ping_request.time, 1577735845610);
    }

    #[test]
    fn test_ping_response_encode() {
        let ping_response = PingResponse {
            time: 1577735845610,
        };

        let mut vec = Vec::new();
        ping_response.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/status/ping_response.dat").to_vec()
        );
    }

    #[test]
    fn test_status_ping_response_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/status/ping_response.dat").to_vec());
        let ping_response = PingResponse::decode(&mut cursor).unwrap();

        assert_eq!(ping_response.time, 1577735845610);
    }

    #[test]
    fn test_status_response_encode() {
        let version = ServerVersion {
            name: String::from("1.15.1"),
            protocol: 575,
        };

        let player = OnlinePlayer {
            id: Uuid::parse_str("2a1e1912-7103-4add-80fc-91ebc346cbce").unwrap(),
            name: String::from("Username"),
        };

        let players = OnlinePlayers {
            online: 10,
            max: 100,
            sample: vec![player],
        };

        let server_status = ServerStatus {
            version,
            description: Message::new(Payload::text("Description")),
            players,
        };

        let status_response = StatusResponse { server_status };

        let mut vec = Vec::new();
        status_response.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/status/status_response.dat").to_vec()
        );
    }

    #[test]
    fn test_status_response_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/status/status_response.dat").to_vec());
        let status_response = StatusResponse::decode(&mut cursor).unwrap();
        let server_status = status_response.server_status;

        let player = OnlinePlayer {
            id: Uuid::parse_str("2a1e1912-7103-4add-80fc-91ebc346cbce").unwrap(),
            name: String::from("Username"),
        };

        assert_eq!(server_status.version.name, String::from("1.15.1"));
        assert_eq!(server_status.version.protocol, 575);
        assert_eq!(server_status.players.max, 100);
        assert_eq!(server_status.players.online, 10);
        assert_eq!(server_status.players.sample, vec![player]);
        assert_eq!(
            server_status.description,
            Message::new(Payload::text("Description"))
        );
    }
}
