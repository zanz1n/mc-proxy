use crate::decoder::Decoder;
use crate::decoder::EnumDecoder;
use crate::encoder::{Encoder, EnumEncoder};
use crate::error::{DecodeError, EncodeError};
use minecraft_protocol_derive::{Decoder, Encoder};
use std::io::{Read, Write};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum LoginServerBoundPacket {
    LoginStart(LoginStart),
    EncryptionResponse(EncryptionResponse),
    LoginPluginResponse(LoginPluginResponse),
    LoginAcknowledged,
}

#[derive(Debug, Clone)]
pub enum LoginClientBoundPacket {
    LoginDisconnect(LoginDisconnect),
    EncryptionRequest(EncryptionRequest),
    LoginSuccess(LoginSuccess),
    SetCompression(SetCompression),
    LoginPluginRequest(LoginPluginRequest),
}

impl EnumEncoder for LoginServerBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            LoginServerBoundPacket::LoginStart(_) => 0x00,
            LoginServerBoundPacket::EncryptionResponse(_) => 0x01,
            LoginServerBoundPacket::LoginPluginResponse(_) => 0x02,
            LoginServerBoundPacket::LoginAcknowledged => 0x03,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            LoginServerBoundPacket::LoginStart(packet) => packet.encode(writer),
            LoginServerBoundPacket::EncryptionResponse(packet) => packet.encode(writer),
            LoginServerBoundPacket::LoginPluginResponse(packet) => packet.encode(writer),
            LoginServerBoundPacket::LoginAcknowledged => Ok(()),
        }
    }
}

impl EnumDecoder for LoginServerBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x00 => {
                let login_start = LoginStart::decode(reader)?;

                Ok(LoginServerBoundPacket::LoginStart(login_start))
            }
            0x01 => {
                let encryption_response = EncryptionResponse::decode(reader)?;

                Ok(LoginServerBoundPacket::EncryptionResponse(
                    encryption_response,
                ))
            }
            0x02 => {
                let login_plugin_response = LoginPluginResponse::decode(reader)?;

                Ok(LoginServerBoundPacket::LoginPluginResponse(
                    login_plugin_response,
                ))
            }
            0x03 => Ok(LoginServerBoundPacket::LoginAcknowledged),
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

impl EnumEncoder for LoginClientBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            LoginClientBoundPacket::LoginDisconnect(_) => 0x00,
            LoginClientBoundPacket::EncryptionRequest(_) => 0x01,
            LoginClientBoundPacket::LoginSuccess(_) => 0x02,
            LoginClientBoundPacket::SetCompression(_) => 0x03,
            LoginClientBoundPacket::LoginPluginRequest(_) => 0x04,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            LoginClientBoundPacket::LoginDisconnect(packet) => packet.encode(writer),
            LoginClientBoundPacket::EncryptionRequest(packet) => packet.encode(writer),
            LoginClientBoundPacket::LoginSuccess(packet) => packet.encode(writer),
            LoginClientBoundPacket::SetCompression(packet) => packet.encode(writer),
            LoginClientBoundPacket::LoginPluginRequest(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for LoginClientBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x00 => {
                let login_disconnect = LoginDisconnect::decode(reader)?;

                Ok(LoginClientBoundPacket::LoginDisconnect(login_disconnect))
            }
            0x01 => {
                let encryption_request = EncryptionRequest::decode(reader)?;

                Ok(LoginClientBoundPacket::EncryptionRequest(
                    encryption_request,
                ))
            }
            0x02 => {
                let login_success = LoginSuccess::decode(reader)?;

                Ok(LoginClientBoundPacket::LoginSuccess(login_success))
            }
            0x03 => {
                let set_compression = SetCompression::decode(reader)?;

                Ok(LoginClientBoundPacket::SetCompression(set_compression))
            }
            0x04 => {
                let login_plugin_request = LoginPluginRequest::decode(reader)?;

                Ok(LoginClientBoundPacket::LoginPluginRequest(
                    login_plugin_request,
                ))
            }
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct LoginStart {
    pub name: String,
    pub uuid: Uuid,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct EncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct LoginPluginResponse {
    #[data_type(with = "var_int")]
    pub message_id: i32,
    pub successful: bool,
    #[data_type(with = "rest")]
    pub data: Vec<u8>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct LoginDisconnect {
    pub reason: String,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct EncryptionRequest {
    #[data_type(max_length = 20)]
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    #[data_type(max_length = 16)]
    pub username: String,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct SetCompression {
    #[data_type(with = "var_int")]
    pub threshold: i32,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct LoginPluginRequest {
    #[data_type(with = "var_int")]
    pub message_id: i32,
    pub channel: String,
    #[data_type(with = "rest")]
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use crate::decoder::Decoder;
    use crate::encoder::Encoder;
    use crate::packet::login::*;
    use std::io::Cursor;
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn test_login_start_packet_encode() {
        let login_start = LoginStart {
            name: String::from("Username"),
            uuid: Uuid::from_str("c676f0db-9695-4fcd-a3cc-d75f129fde7f").unwrap(),
        };

        let mut vec = Vec::new();
        login_start.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/login_start.dat").to_vec()
        );
    }

    #[test]
    fn test_login_start_packet_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/login/login_start.dat").to_vec());
        let login_start = LoginStart::decode(&mut cursor).unwrap();

        assert_eq!(login_start.name, String::from("Username"));
        assert_eq!(
            login_start.uuid,
            Uuid::from_str("c676f0db-9695-4fcd-a3cc-d75f129fde7f").unwrap()
        );
    }

    #[test]
    fn test_encryption_response_encode() {
        let encryption_response = EncryptionResponse {
            shared_secret: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            verify_token: vec![1, 2, 3, 4],
        };

        let mut vec = Vec::new();
        encryption_response.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/encryption_response.dat").to_vec()
        );
    }

    #[test]
    fn test_encryption_response_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/login/encryption_response.dat").to_vec());
        let encryption_response = EncryptionResponse::decode(&mut cursor).unwrap();

        assert_eq!(
            encryption_response.shared_secret,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
        assert_eq!(encryption_response.verify_token, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_login_plugin_response_encode() {
        let login_plugin_response = LoginPluginResponse {
            message_id: 55,
            successful: true,
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        };

        let mut vec = Vec::new();
        login_plugin_response.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/login_plugin_response.dat").to_vec()
        );
    }

    #[test]
    fn test_login_plugin_response_decode() {
        let mut cursor = Cursor::new(
            include_bytes!("../../test/packet/login/login_plugin_response.dat").to_vec(),
        );
        let login_plugin_response = LoginPluginResponse::decode(&mut cursor).unwrap();

        assert_eq!(login_plugin_response.message_id, 55);
        assert!(login_plugin_response.successful);
        assert_eq!(
            login_plugin_response.data,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
    }

    #[test]
    fn test_login_disconnect_encode() {
        let login_disconnect = LoginDisconnect {
            reason: r#"{"text":"Message"}"#.into(),
        };

        let mut vec = Vec::new();
        login_disconnect.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/login_disconnect.dat").to_vec()
        );
    }

    #[test]
    fn test_login_disconnect_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/login/login_disconnect.dat").to_vec());
        let login_disconnect = LoginDisconnect::decode(&mut cursor).unwrap();

        assert_eq!(
            login_disconnect.reason,
            String::from(r#"{"text":"Message"}"#)
        );
    }

    #[test]
    fn test_encryption_request_encode() {
        let encryption_request = EncryptionRequest {
            server_id: String::from("ServerID"),
            public_key: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            verify_token: vec![1, 2, 3, 4],
        };

        let mut vec = Vec::new();
        encryption_request.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/encryption_request.dat").to_vec()
        );
    }

    #[test]
    fn test_encryption_request_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/login/encryption_request.dat").to_vec());
        let encryption_request = EncryptionRequest::decode(&mut cursor).unwrap();

        assert_eq!(encryption_request.server_id, String::from("ServerID"));
        assert_eq!(
            encryption_request.public_key,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
        assert_eq!(encryption_request.verify_token, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_login_success_encode() {
        let login_success = LoginSuccess {
            uuid: Uuid::parse_str("35ee313b-d89a-41b8-b25e-d32e8aff0389").unwrap(),
            username: String::from("Username"),
        };

        let mut vec = Vec::new();
        login_success.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/login_success.dat").to_vec()
        );
    }

    #[test]
    fn test_login_success_decode() {
        let mut cursor =
            Cursor::new(include_bytes!("../../test/packet/login/login_success.dat").to_vec());
        let login_success = LoginSuccess::decode(&mut cursor).unwrap();

        assert_eq!(login_success.username, String::from("Username"));

        assert_eq!(
            login_success.uuid,
            Uuid::parse_str("35ee313b-d89a-41b8-b25e-d32e8aff0389").unwrap()
        );
    }

    #[test]
    fn test_set_compression_encode() {
        let set_compression = SetCompression { threshold: 1 };

        let mut vec = Vec::new();
        set_compression.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/login_set_compression.dat").to_vec()
        );
    }

    #[test]
    fn test_set_compression_decode() {
        let mut cursor = Cursor::new(
            include_bytes!("../../test/packet/login/login_set_compression.dat").to_vec(),
        );
        let set_compression = SetCompression::decode(&mut cursor).unwrap();

        assert_eq!(set_compression.threshold, 1);
    }

    #[test]
    fn test_login_plugin_request_encode() {
        let login_plugin_request = LoginPluginRequest {
            message_id: 55,
            channel: String::from("Channel"),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        };

        let mut vec = Vec::new();
        login_plugin_request.encode(&mut vec).unwrap();

        assert_eq!(
            vec,
            include_bytes!("../../test/packet/login/login_plugin_request.dat").to_vec()
        );
    }

    #[test]
    fn test_login_plugin_request_decode() {
        let mut cursor = Cursor::new(
            include_bytes!("../../test/packet/login/login_plugin_request.dat").to_vec(),
        );
        let login_plugin_request = LoginPluginRequest::decode(&mut cursor).unwrap();

        assert_eq!(login_plugin_request.message_id, 55);
        assert_eq!(login_plugin_request.channel, String::from("Channel"));
        assert_eq!(
            login_plugin_request.data,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
    }
}
