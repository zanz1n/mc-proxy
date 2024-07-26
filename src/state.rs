use crate::repository::{
    ip_bans::SqlxIpBansRepository, kv::SqlxKeyValueRepository, user_bans::SqlxUserBansRepository,
    whitelist::SqlxWhitelistRepository, DB,
};
use minecraft_protocol::{
    codec::{
        client::{ClientPacket, ClientPacketCodec},
        server::{ServerPacket, ServerPacketCodec},
        ProtocolState,
    },
    data::chat::Message,
    error::DecodeError,
};
use std::{collections::HashMap, future::Future};
use tokio::sync::{RwLock, RwLockReadGuard};
use uuid::Uuid;

pub struct GlobalSharedState {
    server_description: RwLock<Message>,
    pub ip_bans: SqlxIpBansRepository<DB>,
    pub user_bans: SqlxUserBansRepository<DB>,
    pub whitelist: SqlxWhitelistRepository<DB, SqlxKeyValueRepository<DB>>,
    online_players: RwLock<HashMap<String, Uuid>>,
}

impl GlobalSharedState {
    pub fn new(
        server_description: Message,
        ip_bans: SqlxIpBansRepository<DB>,
        user_bans: SqlxUserBansRepository<DB>,
        whitelist: SqlxWhitelistRepository<DB, SqlxKeyValueRepository<DB>>,
    ) -> GlobalSharedState {
        GlobalSharedState {
            server_description: RwLock::new(server_description),
            ip_bans,
            user_bans,
            whitelist,
            online_players: RwLock::new(HashMap::new()),
        }
    }

    pub async fn server_description(&self) -> Message {
        self.server_description.read().await.clone()
    }

    pub async fn remove_online_player(&self, name: &str) {
        self.online_players.write().await.remove(name);
    }

    pub async fn set_server_description(&self, server_description: Message) {
        let mut lock = self.server_description.write().await;
        *lock = server_description;
    }

    pub async fn add_online_player(&self, name: String, uuid: Uuid) {
        let mut lock = self.online_players.write().await;
        lock.insert(name, uuid);
    }

    pub async fn exists_online_player(&self, name: &str) -> bool {
        self.online_players.read().await.get(name).is_some()
    }

    #[inline]
    pub fn read_online_players(
        &self,
    ) -> impl Future<Output = RwLockReadGuard<HashMap<String, Uuid>>> + Send {
        self.online_players.read()
    }
}

pub struct PostLoginInformation {
    pub username: String,
    pub uuid: Uuid,
}

pub struct ConnectionSharedState {
    pub protocol_version: i32,
    pub login_info: RwLock<Option<PostLoginInformation>>,
    client_codec: RwLock<ClientPacketCodec>,
    server_codec: RwLock<ServerPacketCodec>,
}

impl ConnectionSharedState {
    #[inline]
    pub fn new(protocol_version: i32) -> Self {
        Self {
            protocol_version,
            login_info: RwLock::new(None),
            client_codec: RwLock::new(ClientPacketCodec::new()),
            server_codec: RwLock::new(ServerPacketCodec::new()),
        }
    }

    pub async fn login_username(&self) -> Option<String> {
        self.login_info
            .read()
            .await
            .as_ref()
            .map(|v| v.username.clone())
    }

    pub async fn current_state(&self) -> ProtocolState {
        self.client_codec.read().await.state()
    }

    pub async fn set_state(&self, state: ProtocolState) {
        self.client_codec.write().await.set_state(state);
        self.server_codec.write().await.set_state(state);
    }

    pub async fn set_compression(&self, threshold: usize) {
        self.client_codec.write().await.set_compression(threshold);
        self.server_codec.write().await.set_compression(threshold);
    }

    pub async fn decode_client(&self, data: &[u8]) -> Result<Option<ClientPacket>, DecodeError> {
        self.client_codec.write().await.decode(data)
    }

    pub async fn decode_server(&self, data: &[u8]) -> Result<Option<ServerPacket>, DecodeError> {
        self.server_codec.write().await.decode(data)
    }
}
