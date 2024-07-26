use super::CommandResult;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRequestMessage {
    pub id: Uuid,
    pub command: CommandRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum CommandRequest {
    // User bans
    BanPlayer(BanPlayerRequest),
    UnbanPlayer(UsernameMessage),
    IsPlayerBanned(UsernameMessage),
    GetPlayerBans,

    // IP Bans
    BanIp(BanIpRequest),
    UnbanIp(IpMessage),
    IsIpBanned(IpMessage),
    GetIpBans,

    // Whitelist
    SetWhitelistEnabled(SetWhitelistEnabled),
    IsWhitelistEnabled,
    IsWhitelisted(UsernameMessage),
    WhitelistAddPlayer(UsernameMessage),
    WhitelistRemovePlayer(UsernameMessage),
    WhitelistGetAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsernameMessage {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanPlayerRequest {
    pub username: String,
    /// The time should be in milliseconds
    pub duration: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BanIpRequest {
    pub ip: IpAddr,
    /// The time should be in milliseconds
    pub duration: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpMessage {
    pub ip: IpAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetWhitelistEnabled {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandResponseMessage {
    pub id: Uuid,
    pub result: CommandResult<CommandResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum CommandResponse {
    // User bans
    BanPlayer,
    UnbanPlayer(ChangedMessage),
    IsPlayerBanned(IsBannedMessage),
    GetPlayerBans(GetPlayerBansResponse),

    // IP Bans
    BanIp,
    UnbanIp(ChangedMessage),
    IsIpBanned(IsBannedMessage),
    GetIpBans(GetIpBansResponse),

    // Whitelist
    SetWhitelistEnabled(ChangedMessage),
    IsWhitelistEnabled(IsWhitelistEnabledResponse),
    IsWhitelisted(IsWhitelistedResponse),
    WhitelistAddPlayer(ChangedMessage),
    WhitelistRemovePlayer(ChangedMessage),
    WhitelistGetAll(WhitelistGetAllResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangedMessage {
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsBannedMessage {
    pub banned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetPlayerBansResponse {
    pub bans: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetIpBansResponse {
    pub bans: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsWhitelistEnabledResponse {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsWhitelistedResponse {
    pub whitelisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhitelistGetAllResponse {
    pub whitelist: Vec<String>,
}
