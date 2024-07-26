use crate::nbt::decode::TagDecodeError;
use serde_json::error::Error as JsonError;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::string::FromUtf8Error;
use uuid::Error as UuidParseError;

/// Possible errors while encoding packet.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// String length can't be more than provided value.
    #[error("String too long: got {length} while max length is {max_length}")]
    StringTooLong {
        /// String length.
        length: usize,
        /// Max string length.
        max_length: u16,
    },
    #[error("Io error: {io_error}")]
    IOError { io_error: IoError },
    #[error("Failed to encode json data: {json_error}")]
    JsonError { json_error: JsonError },
}

impl From<IoError> for EncodeError {
    #[inline]
    fn from(io_error: IoError) -> Self {
        EncodeError::IOError { io_error }
    }
}

impl From<JsonError> for EncodeError {
    #[inline]
    fn from(json_error: JsonError) -> Self {
        EncodeError::JsonError { json_error }
    }
}

/// Possible errors while decoding packet.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// Packet was not recognized. Invalid data or wrong protocol version.
    #[error("Unknown packet type: {type_id}")]
    UnknownPacketType { type_id: u8 },
    /// String length can't be more than provided value.
    #[error("String too long: got {length} while max length is {max_length}")]
    StringTooLong {
        /// String length.
        length: usize,
        /// Max string length.
        max_length: u16,
    },
    #[error("Io error: {io_error}")]
    IOError {
        #[from]
        io_error: IoError,
    },
    #[error("Invalid json data: {json_error}")]
    JsonError {
        #[from]
        json_error: JsonError,
    },
    /// Byte array was not recognized as valid UTF-8 string.
    #[error("Invalid utf8 string: {utf8_error}")]
    Utf8Error {
        #[from]
        utf8_error: FromUtf8Error,
    },
    /// Boolean are parsed from byte. Valid byte value are 0 or 1.
    #[error("The value is not a valid boolean")]
    NonBoolValue,
    #[error("Invalid uuid: {uuid_parse_error}")]
    UuidParseError {
        #[from]
        uuid_parse_error: UuidParseError,
    },
    /// Type id was not parsed as valid enum value.
    #[error("Unknown enum: got type id {type_id}")]
    UnknownEnumType { type_id: usize },
    #[error("Failed to decode nbt tag: {tag_decode_error}")]
    TagDecodeError {
        #[from]
        tag_decode_error: TagDecodeError,
    },
    #[error("Varint length exceeds {max_bytes} bytes")]
    VarIntTooLong { max_bytes: usize },
    #[error("Data was sent during handshake state")]
    DataSentDuringHandshake,
    #[error("The provided packet length is invalid")]
    InvalidPacketLength,
}

impl DecodeError {
    pub fn is_eof_error(&self) -> bool {
        if let DecodeError::IOError { io_error: error } = self {
            if matches!(error.kind(), IoErrorKind::UnexpectedEof) {
                return true;
            }
        }

        false
    }
}
