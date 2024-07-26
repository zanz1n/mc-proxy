use crate::utils::read_packet;
use minecraft_protocol::{
    codec::ProtocolState,
    decoder::Decoder,
    error::DecodeError,
    packet::handshake::{Handshake, HandshakeServerBoundPacket},
};
use std::io::Cursor;
use tokio::io::AsyncRead;

pub async fn handle_handshake<R: AsyncRead + Unpin + Send>(
    client_read: &mut R,
) -> Result<Handshake, DecodeError> {
    let vec = read_packet(client_read, false)
        .await?
        .ok_or(DecodeError::InvalidPacketLength)?;
    let mut cursor = Cursor::new(vec);

    let packet = HandshakeServerBoundPacket::decode(&mut cursor)?;

    tracing::trace!(
        current_state = ?ProtocolState::Handshake,
        ?packet,
        "Incomming client packet",
    );

    let handshake_packet = match packet {
        HandshakeServerBoundPacket::Handshake(v) => v,
    };

    Ok(handshake_packet)
}
