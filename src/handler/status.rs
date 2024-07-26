use crate::{
    state::GlobalSharedState,
    utils::{read_packet, write_packet},
};
use minecraft_protocol::{
    codec::ProtocolState,
    data::server_status::{OnlinePlayer, OnlinePlayers, ServerStatus, ServerVersion},
    decoder::Decoder,
    error::DecodeError,
    packet::{
        handshake::Handshake,
        status::{PingResponse, StatusClientBoundPacket, StatusResponse, StatusServerBoundPacket},
    },
};
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn handle_status<C: AsyncRead + AsyncWrite + Unpin + Send>(
    global_state: &GlobalSharedState,
    handshake_data: &Handshake,
    conn: &mut C,
) -> Result<(), DecodeError> {
    let current_state = ProtocolState::Status;

    loop {
        let vec = match read_packet(conn, false).await? {
            Some(v) => v,
            None => break,
        };
        let mut cursor = Cursor::new(vec);

        let packet = StatusServerBoundPacket::decode(&mut cursor)?;

        tracing::trace!(?current_state, ?packet, "Incomming client packet");

        match packet {
            StatusServerBoundPacket::StatusRequest => {
                let description = global_state.server_description().await;
                let online_players = global_state.read_online_players().await;

                let online_count = online_players.len();

                let online_sample = online_players
                    .iter()
                    .map(|(key, value)| OnlinePlayer {
                        id: value.clone(),
                        name: key.clone(),
                    })
                    .collect();

                drop(online_players);

                let packet = StatusClientBoundPacket::StatusResponse(StatusResponse {
                    server_status: ServerStatus {
                        description,
                        players: OnlinePlayers {
                            max: 0,
                            online: online_count.try_into().unwrap(),
                            sample: online_sample,
                        },
                        version: ServerVersion {
                            name: format!("Basileia Proxy {}", env!("CARGO_PKG_VERSION")),
                            protocol: handshake_data.protocol_version.try_into().unwrap(),
                        },
                    },
                });

                write_packet(conn, &packet).await?;
                tracing::debug!("Status connection responded");
            }
            StatusServerBoundPacket::PingRequest(req) => {
                write_packet(
                    conn,
                    &StatusClientBoundPacket::PingResponse(PingResponse { time: req.time }),
                )
                .await?;

                break;
            }
        }
    }

    Ok(())
}
