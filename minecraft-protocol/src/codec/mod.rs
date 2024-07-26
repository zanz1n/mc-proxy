pub mod client;
pub mod codec;
pub mod server;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProtocolState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}
