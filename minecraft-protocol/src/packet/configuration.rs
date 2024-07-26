use crate::{
    data::chat::Message,
    decoder::{Decoder, EnumDecoder},
    encoder::{Encoder, EnumEncoder},
    error::{DecodeError, EncodeError},
    nbt::CompoundTag,
};
use minecraft_protocol_derive::{Decoder, Encoder};
use std::io::{Read, Write};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum ConfigServerBoundPacket {
    ClientInformation(ClientInformation),
    ServerBoundPluginMessage(ServerBoundPluginMessage),
    AcknowledgeFinishConfiguration,
    ServerBoundKeepAlive(ServerBoundKeepAlive),
    Pong(Pong),
    ResourcePackResponse(ResourcePackResponse),
}

#[derive(Debug, Clone)]
pub enum ConfigClientBoundPaket {
    ClientBoundPluginMessage(ClientBoundPluginMessage),
    ConfigDisconnect(ConfigDisconnect),
    FinishConfiguration,
    ClientboundKeepAlive(ClientboundKeepAlive),
    Ping(Ping),
    RegistryData(RegistryData),
    RemoveResourcePack(RemoveResourcePack),
    AddResourcePack(AddResourcePack),
    FeatureFlags(FeatureFlags),
    UpdateTags(UpdateTags),
}

impl EnumEncoder for ConfigServerBoundPacket {
    fn get_type_id(&self) -> u8 {
        match self {
            ConfigServerBoundPacket::ClientInformation(_) => 0x00,
            ConfigServerBoundPacket::ServerBoundPluginMessage(_) => 0x01,
            ConfigServerBoundPacket::AcknowledgeFinishConfiguration => 0x02,
            ConfigServerBoundPacket::ServerBoundKeepAlive(_) => 0x03,
            ConfigServerBoundPacket::Pong(_) => 0x04,
            ConfigServerBoundPacket::ResourcePackResponse(_) => 0x05,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            ConfigServerBoundPacket::ClientInformation(packet) => packet.encode(writer),
            ConfigServerBoundPacket::ServerBoundPluginMessage(packet) => packet.encode(writer),
            ConfigServerBoundPacket::AcknowledgeFinishConfiguration => Ok(()),
            ConfigServerBoundPacket::ServerBoundKeepAlive(packet) => packet.encode(writer),
            ConfigServerBoundPacket::Pong(packet) => packet.encode(writer),
            ConfigServerBoundPacket::ResourcePackResponse(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for ConfigServerBoundPacket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x00 => {
                let client_information = ClientInformation::decode(reader)?;

                Ok(ConfigServerBoundPacket::ClientInformation(
                    client_information,
                ))
            }
            0x01 => {
                let plugin_message = ServerBoundPluginMessage::decode(reader)?;

                Ok(ConfigServerBoundPacket::ServerBoundPluginMessage(
                    plugin_message,
                ))
            }
            0x02 => Ok(ConfigServerBoundPacket::AcknowledgeFinishConfiguration),
            0x03 => {
                let keep_alive = ServerBoundKeepAlive::decode(reader)?;

                Ok(ConfigServerBoundPacket::ServerBoundKeepAlive(keep_alive))
            }
            0x04 => {
                let pong = Pong::decode(reader)?;

                Ok(ConfigServerBoundPacket::Pong(pong))
            }
            0x05 => {
                let resource_pack_response = ResourcePackResponse::decode(reader)?;

                Ok(ConfigServerBoundPacket::ResourcePackResponse(
                    resource_pack_response,
                ))
            }
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

impl EnumEncoder for ConfigClientBoundPaket {
    fn get_type_id(&self) -> u8 {
        match self {
            ConfigClientBoundPaket::ClientBoundPluginMessage(_) => 0x00,
            ConfigClientBoundPaket::ConfigDisconnect(_) => 0x01,
            ConfigClientBoundPaket::FinishConfiguration => 0x02,
            ConfigClientBoundPaket::ClientboundKeepAlive(_) => 0x03,
            ConfigClientBoundPaket::Ping(_) => 0x04,
            ConfigClientBoundPaket::RegistryData(_) => 0x05,
            ConfigClientBoundPaket::RemoveResourcePack(_) => 0x06,
            ConfigClientBoundPaket::AddResourcePack(_) => 0x07,
            ConfigClientBoundPaket::FeatureFlags(_) => 0x08,
            ConfigClientBoundPaket::UpdateTags(_) => 0x09,
        }
    }

    fn encode<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        match self {
            ConfigClientBoundPaket::ClientBoundPluginMessage(packet) => packet.encode(writer),
            ConfigClientBoundPaket::ConfigDisconnect(packet) => packet.encode(writer),
            ConfigClientBoundPaket::FinishConfiguration => Ok(()),
            ConfigClientBoundPaket::ClientboundKeepAlive(packet) => packet.encode(writer),
            ConfigClientBoundPaket::Ping(packet) => packet.encode(writer),
            ConfigClientBoundPaket::RegistryData(packet) => packet.encode(writer),
            ConfigClientBoundPaket::RemoveResourcePack(packet) => packet.encode(writer),
            ConfigClientBoundPaket::AddResourcePack(packet) => packet.encode(writer),
            ConfigClientBoundPaket::FeatureFlags(packet) => packet.encode(writer),
            ConfigClientBoundPaket::UpdateTags(packet) => packet.encode(writer),
        }
    }
}

impl EnumDecoder for ConfigClientBoundPaket {
    type Output = Self;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError> {
        match type_id {
            0x00 => {
                let plugin_message = ClientBoundPluginMessage::decode(reader)?;

                Ok(ConfigClientBoundPaket::ClientBoundPluginMessage(
                    plugin_message,
                ))
            }
            0x01 => {
                let config_disconnect = ConfigDisconnect::decode(reader)?;

                Ok(ConfigClientBoundPaket::ConfigDisconnect(config_disconnect))
            }
            0x02 => Ok(ConfigClientBoundPaket::FinishConfiguration),
            0x03 => {
                let keep_alive = ClientboundKeepAlive::decode(reader)?;

                Ok(ConfigClientBoundPaket::ClientboundKeepAlive(keep_alive))
            }
            0x04 => {
                let ping = Ping::decode(reader)?;

                Ok(ConfigClientBoundPaket::Ping(ping))
            }
            0x05 => {
                let registry_data = RegistryData::decode(reader)?;

                Ok(ConfigClientBoundPaket::RegistryData(registry_data))
            }
            0x06 => {
                let remove_resource_pack = RemoveResourcePack::decode(reader)?;

                Ok(ConfigClientBoundPaket::RemoveResourcePack(
                    remove_resource_pack,
                ))
            }
            0x07 => {
                let add_resource_pack = AddResourcePack::decode(reader)?;

                Ok(ConfigClientBoundPaket::AddResourcePack(add_resource_pack))
            }
            0x08 => {
                let feature_flags = FeatureFlags::decode(reader)?;

                Ok(ConfigClientBoundPaket::FeatureFlags(feature_flags))
            }
            0x09 => {
                let update_tags = UpdateTags::decode(reader)?;

                Ok(ConfigClientBoundPaket::UpdateTags(update_tags))
            }
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct ClientInformation {
    #[data_type(max_length = 16)]
    pub locale: String,
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    pub chat_colors: bool,
    pub display_skin_parts: u8,
    #[data_type(with = "var_int")]
    pub main_hand: i32,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[derive(Encoder, Decoder, Debug, Clone)]
#[data_type(with = "var_int")]
pub enum ChatMode {
    Enabled,
    CommandsOnly,
    Hidden,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct ServerBoundPluginMessage {
    pub channel: String,
    #[data_type(with = "rest")]
    pub data: Vec<u8>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct ServerBoundKeepAlive {
    pub id: u64,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct Pong {
    pub id: u32,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct ResourcePackResponse {
    pub uuid: Uuid,
    pub result: ResourcePackResult,
}

#[derive(Encoder, Decoder, Debug, Clone)]
#[data_type(with = "var_int")]
pub enum ResourcePackResult {
    SuccessfullyDownloaded,
    Declined,
    DownloadFailed,
    Accepted,
    Downloaded,
    InvalidUrl,
    ReloadFailed,
    Discarded,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct ClientBoundPluginMessage {
    pub channel: String,
    #[data_type(with = "rest")]
    pub data: Vec<u8>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct ConfigDisconnect {
    pub reason: Message,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct ClientboundKeepAlive {
    pub id: u64,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct Ping {
    pub id: u32,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct RegistryData {
    pub data: CompoundTag,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct RemoveResourcePack {
    #[data_type(with = "bool_option")]
    uuid: Option<Uuid>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct AddResourcePack {
    pub uuid: Uuid,
    #[data_type(max_length = 32767)]
    pub url: String,
    #[data_type(max_length = 40)]
    pub hash: String,
    pub forced: bool,
    #[data_type(with = "bool_option")]
    pub prompt_message: Option<Message>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct FeatureFlags {
    /// The non-decoded representation of the feature flags array
    ///
    /// TODO: Implement feature flags decoding
    #[data_type(with = "rest")]
    pub feature_flags: Vec<u8>,
}

#[derive(Encoder, Decoder, Debug, Clone)]
pub struct UpdateTags {
    /// The non-decoded representation of the tags array
    ///
    /// TODO: Implement tags decoding
    #[data_type(with = "rest")]
    pub tags: Vec<u8>,
}
