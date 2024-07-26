use super::{codec::MinecraftCodec, ProtocolState};
use crate::{
    encoder::EnumEncoder,
    error::DecodeError,
    packet::{
        configuration::ConfigServerBoundPacket, game::GameServerBoundPacket,
        handshake::HandshakeServerBoundPacket, login::LoginServerBoundPacket,
        status::StatusServerBoundPacket,
    },
};

pub struct ClientPacketCodec {
    state: ProtocolState,
    codec: MinecraftCodec,
}

impl Default for ClientPacketCodec {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ClientPacketCodec {
    #[inline]
    pub fn new() -> Self {
        Self {
            state: ProtocolState::Handshake,
            codec: MinecraftCodec::new(),
        }
    }

    #[inline]
    pub fn state(&self) -> ProtocolState {
        self.state
    }

    #[inline]
    pub fn set_state(&mut self, state: ProtocolState) {
        self.state = state
    }

    #[inline]
    pub fn set_compression(&mut self, threshold: usize) {
        self.codec.enable_compression(threshold)
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Option<ClientPacket>, DecodeError> {
        self.codec.accept(data);
        match self.state {
            ProtocolState::Handshake => self
                .codec
                .next_packet::<HandshakeServerBoundPacket>()
                .map(|opt| opt.map(ClientPacket::from)),
            ProtocolState::Status => self
                .codec
                .next_packet::<StatusServerBoundPacket>()
                .map(|opt| opt.map(ClientPacket::from)),
            ProtocolState::Login => self
                .codec
                .next_packet::<LoginServerBoundPacket>()
                .map(|opt| opt.map(ClientPacket::from)),
            ProtocolState::Configuration => self
                .codec
                .next_packet::<ConfigServerBoundPacket>()
                .map(|opt| opt.map(ClientPacket::from)),
            ProtocolState::Play => self
                .codec
                .next_packet::<GameServerBoundPacket>()
                .map(|opt| opt.map(ClientPacket::from)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClientPacket {
    Handshake(HandshakeServerBoundPacket),
    Status(StatusServerBoundPacket),
    Login(LoginServerBoundPacket),
    Configuration(ConfigServerBoundPacket),
    Game(GameServerBoundPacket),
}

impl ClientPacket {
    #[inline]
    pub fn get_type_id(&self) -> u8 {
        match self {
            ClientPacket::Handshake(packet) => packet.get_type_id(),
            ClientPacket::Status(packet) => packet.get_type_id(),
            ClientPacket::Login(packet) => packet.get_type_id(),
            ClientPacket::Configuration(packet) => packet.get_type_id(),
            ClientPacket::Game(packet) => packet.get_type_id(),
        }
    }
}

impl From<HandshakeServerBoundPacket> for ClientPacket {
    #[inline]
    fn from(packet: HandshakeServerBoundPacket) -> Self {
        ClientPacket::Handshake(packet)
    }
}

impl From<StatusServerBoundPacket> for ClientPacket {
    #[inline]
    fn from(packet: StatusServerBoundPacket) -> Self {
        ClientPacket::Status(packet)
    }
}

impl From<LoginServerBoundPacket> for ClientPacket {
    #[inline]
    fn from(packet: LoginServerBoundPacket) -> Self {
        ClientPacket::Login(packet)
    }
}

impl From<ConfigServerBoundPacket> for ClientPacket {
    #[inline]
    fn from(value: ConfigServerBoundPacket) -> Self {
        ClientPacket::Configuration(value)
    }
}

impl From<GameServerBoundPacket> for ClientPacket {
    #[inline]
    fn from(packet: GameServerBoundPacket) -> Self {
        ClientPacket::Game(packet)
    }
}
