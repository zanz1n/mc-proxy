use super::{
    server::{
        ChangedMessage, CommandRequest, CommandRequestMessage, CommandResponse,
        CommandResponseMessage, GetIpBansResponse, GetPlayerBansResponse, IpMessage,
        IsBannedMessage, IsWhitelistEnabledResponse, IsWhitelistedResponse, UsernameMessage,
        WhitelistGetAllResponse,
    },
    CommandError,
};
use crate::{
    repository::{
        ip_bans::IpBansRepository, user_bans::UserBansRepository, whitelist::WhitelistRepository,
    },
    state::GlobalSharedState,
};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

pub async fn proxy_command_events(
    state: &GlobalSharedState,
    mut request_recv: mpsc::Receiver<Vec<u8>>,
    response_sender: mpsc::Sender<Vec<u8>>,
) {
    loop {
        let request = match request_recv.recv().await {
            Some(v) => v,
            None => break,
        };
        let response = handle_command_data(state, &request).await;
        if response_sender.send(response).await.is_err() {
            break;
        }
    }
}

pub async fn handle_command_data(state: &GlobalSharedState, command_data: &[u8]) -> Vec<u8> {
    match serde_json::from_slice::<'_, CommandRequestMessage>(&command_data) {
        Ok(req) => {
            tracing::info!(id = %req.id, command = ?req.command, "Incomming command");

            let start = Instant::now();
            let res = handle_command(state, req.command).await;

            let v = CommandResponseMessage {
                id: req.id,
                result: res.into(),
            };

            let res = serde_json::to_vec(&v).unwrap_or_else(|error| {
                tracing::error!(%error, "Failed to encode command response");

                serde_json::to_vec(&CommandResponseMessage {
                    id: req.id,
                    result: Err(CommandError::CommandEncodeError(error)).into(),
                })
                .unwrap_or_else(|_| Vec::new())
            });

            let took = Instant::now() - start;
            tracing::info!(id = %req.id, ?took, "Handled command");

            res
        }
        Err(error) => {
            tracing::error!(%error, "Failed to decode incomming command");

            serde_json::to_vec(&CommandResponseMessage {
                id: Uuid::nil(),
                result: Err(CommandError::CommandDecodeError(error)).into(),
            })
            .unwrap_or_else(|_| Vec::new())
        }
    }
}

pub async fn handle_command(
    state: &GlobalSharedState,
    command: CommandRequest,
) -> Result<CommandResponse, CommandError> {
    match command {
        CommandRequest::BanPlayer(ban_player) => {
            let duration = ban_player.duration.map(Duration::from_millis);

            state
                .user_bans
                .add_ban(&ban_player.username, duration, ban_player.reason)
                .await?;

            Ok(CommandResponse::BanPlayer)
        }
        CommandRequest::UnbanPlayer(UsernameMessage { username }) => {
            let changed = state.user_bans.remove_ban(&username).await?.is_some();

            Ok(CommandResponse::UnbanPlayer(ChangedMessage { changed }))
        }
        CommandRequest::IsPlayerBanned(UsernameMessage { username }) => {
            let banned = state.user_bans.is_banned(&username).await?.is_some();

            Ok(CommandResponse::IsPlayerBanned(IsBannedMessage { banned }))
        }
        CommandRequest::GetPlayerBans => {
            let bans = state
                .user_bans
                .get_bans()
                .await?
                .into_iter()
                .map(|v| v.username)
                .collect();

            Ok(CommandResponse::GetPlayerBans(GetPlayerBansResponse {
                bans,
            }))
        }
        CommandRequest::BanIp(ban_ip) => {
            let duration = ban_ip.duration.map(Duration::from_millis);

            state
                .ip_bans
                .add_ban(ban_ip.ip, duration, ban_ip.reason)
                .await?;

            Ok(CommandResponse::BanIp)
        }
        CommandRequest::UnbanIp(IpMessage { ip }) => {
            let changed = state.ip_bans.remove_ban(ip).await?.is_some();

            Ok(CommandResponse::UnbanIp(ChangedMessage { changed }))
        }
        CommandRequest::IsIpBanned(IpMessage { ip }) => {
            let banned = state.ip_bans.is_banned(ip).await?.is_some();

            Ok(CommandResponse::IsIpBanned(IsBannedMessage { banned }))
        }
        CommandRequest::GetIpBans => {
            let bans = state
                .ip_bans
                .get_bans()
                .await?
                .into_iter()
                .map(|v| v.ip.to_string())
                .collect();

            Ok(CommandResponse::GetIpBans(GetIpBansResponse { bans }))
        }
        CommandRequest::SetWhitelistEnabled(set_enabled) => {
            let before_enabled = state.whitelist.is_enabled().await?;
            state.whitelist.set_enabled(set_enabled.enabled).await?;

            Ok(CommandResponse::SetWhitelistEnabled(ChangedMessage {
                changed: before_enabled == set_enabled.enabled,
            }))
        }
        CommandRequest::IsWhitelistEnabled => {
            let enabled = state.whitelist.is_enabled().await?;

            Ok(CommandResponse::IsWhitelistEnabled(
                IsWhitelistEnabledResponse { enabled },
            ))
        }
        CommandRequest::IsWhitelisted(UsernameMessage { username }) => {
            let whitelisted = state.whitelist.is_whitelisted(&username).await?;

            Ok(CommandResponse::IsWhitelisted(IsWhitelistedResponse {
                whitelisted,
            }))
        }
        CommandRequest::WhitelistAddPlayer(UsernameMessage { username }) => {
            let result = state.whitelist.add(&username).await?;

            Ok(CommandResponse::WhitelistAddPlayer(ChangedMessage {
                changed: result.is_changed(),
            }))
        }
        CommandRequest::WhitelistRemovePlayer(UsernameMessage { username }) => {
            let result = state.whitelist.remove(&username).await?;

            Ok(CommandResponse::WhitelistRemovePlayer(ChangedMessage {
                changed: result.is_changed(),
            }))
        }
        CommandRequest::WhitelistGetAll => {
            let whitelist = state.whitelist.get_all().await?;

            Ok(CommandResponse::WhitelistGetAll(WhitelistGetAllResponse {
                whitelist,
            }))
        }
    }
}
