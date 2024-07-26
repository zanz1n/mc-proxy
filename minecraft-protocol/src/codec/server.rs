use super::{codec::MinecraftCodec, ProtocolState};
use crate::{
    encoder::EnumEncoder,
    error::DecodeError,
    packet::{
        configuration::ConfigClientBoundPaket, game::GameClientBoundPacket,
        login::LoginClientBoundPacket, status::StatusClientBoundPacket,
    },
};

pub struct ServerPacketCodec {
    state: ProtocolState,
    codec: MinecraftCodec,
}

impl Default for ServerPacketCodec {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ServerPacketCodec {
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

    pub fn decode(&mut self, data: &[u8]) -> Result<Option<ServerPacket>, DecodeError> {
        self.codec.accept(data);
        match self.state {
            ProtocolState::Handshake => Err(DecodeError::DataSentDuringHandshake),
            ProtocolState::Status => self
                .codec
                .next_packet::<StatusClientBoundPacket>()
                .map(|opt| opt.map(ServerPacket::from)),
            ProtocolState::Login => self
                .codec
                .next_packet::<LoginClientBoundPacket>()
                .map(|opt| opt.map(ServerPacket::from)),
            ProtocolState::Configuration => self
                .codec
                .next_packet::<ConfigClientBoundPaket>()
                .map(|opt| opt.map(ServerPacket::from)),
            ProtocolState::Play => self
                .codec
                .next_packet::<GameClientBoundPacket>()
                .map(|opt| opt.map(ServerPacket::from)),
        }
    }

    pub fn encode(&mut self, packet: &ServerPacket, buffer: &mut Vec<u8>) {
        match packet {
            ServerPacket::Status(packet) => self.codec.encode(packet, buffer).unwrap(),
            ServerPacket::Login(packet) => self.codec.encode(packet, buffer).unwrap(),
            ServerPacket::Configuration(packet) => self.codec.encode(packet, buffer).unwrap(),
            ServerPacket::Play(packet) => self.codec.encode(packet, buffer).unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServerPacket {
    Status(StatusClientBoundPacket),
    Login(LoginClientBoundPacket),
    Configuration(ConfigClientBoundPaket),
    Play(GameClientBoundPacket),
}

impl ServerPacket {
    #[inline]
    pub fn id(&self) -> u8 {
        match self {
            ServerPacket::Status(packet) => packet.get_type_id(),
            ServerPacket::Login(packet) => packet.get_type_id(),
            ServerPacket::Configuration(packet) => packet.get_type_id(),
            ServerPacket::Play(packet) => packet.get_type_id(),
        }
    }
}

impl From<StatusClientBoundPacket> for ServerPacket {
    #[inline]
    fn from(packet: StatusClientBoundPacket) -> Self {
        ServerPacket::Status(packet)
    }
}

impl From<LoginClientBoundPacket> for ServerPacket {
    #[inline]
    fn from(packet: LoginClientBoundPacket) -> Self {
        ServerPacket::Login(packet)
    }
}

impl From<ConfigClientBoundPaket> for ServerPacket {
    #[inline]
    fn from(value: ConfigClientBoundPaket) -> Self {
        ServerPacket::Configuration(value)
    }
}

impl From<GameClientBoundPacket> for ServerPacket {
    #[inline]
    fn from(packet: GameClientBoundPacket) -> Self {
        ServerPacket::Play(packet)
    }
}
