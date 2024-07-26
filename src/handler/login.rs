use crate::{
    errors::AppError,
    repository::user_bans::UserBansRepository,
    state::GlobalSharedState,
    utils::{read_packet, write_packet},
};
use minecraft_protocol::{
    codec::ProtocolState,
    decoder::Decoder,
    packet::login::{LoginClientBoundPacket, LoginDisconnect, LoginServerBoundPacket, LoginStart},
};
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncWrite};

const PLAYER_EXISTS_MSG: &'static str =
    r#"{"text":"There is already a logged in player with this username"}"#;

pub async fn handle_login_start<C: AsyncRead + AsyncWrite + Unpin + Send>(
    global_state: &GlobalSharedState,
    conn: &mut C,
) -> Result<Option<LoginStart>, AppError> {
    let vec = match read_packet(conn, false).await? {
        Some(v) => v,
        None => return Ok(None),
    };

    let mut cursor = Cursor::new(vec);

    let packet = LoginServerBoundPacket::decode(&mut cursor)?;

    tracing::trace!(
        current_state = ?ProtocolState::Login,
        ?packet,
        "Incomming client packet",
    );

    if let LoginServerBoundPacket::LoginStart(login_start) = packet {
        let exists = global_state.exists_online_player(&login_start.name).await;

        if exists {
            tracing::info!(
                username = login_start.name,
                "A player with this username is already connected"
            );

            let packet = LoginClientBoundPacket::LoginDisconnect(LoginDisconnect {
                reason: PLAYER_EXISTS_MSG.into(),
            });
            let _ = write_packet(conn, &packet).await.map_err(|error| {
                tracing::warn!(%error, "Failed to send disconnect message to client");
            });
        } else {
            let ban = global_state.user_bans.is_banned(&login_start.name).await?;

            if let Some(ban) = ban {
                let reason = if let Some(reason) = ban.reason {
                    format!("Banned! Reason: {reason}")
                } else {
                    "Banned!".into()
                };

                let packet = LoginClientBoundPacket::LoginDisconnect(LoginDisconnect { reason });
                let _ = write_packet(conn, &packet).await.map_err(|error| {
                    tracing::warn!(%error, "Failed to send disconnect message to client");
                });

                return Ok(None);
            }
            return Ok(Some(login_start));
        }
    }

    Ok(None)
}
