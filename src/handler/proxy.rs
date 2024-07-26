use crate::{
    state::{ConnectionSharedState, GlobalSharedState, PostLoginInformation},
    utils::{read_packet, write_packet},
};
use minecraft_protocol::{
    codec::{client::ClientPacket, server::ServerPacket, ProtocolState},
    error::DecodeError,
    packet::{
        configuration::{ConfigClientBoundPaket, ConfigServerBoundPacket},
        game::{GameClientBoundPacket, GameServerBoundPacket, PlayPluginMessage},
        login::{LoginClientBoundPacket, LoginServerBoundPacket},
    },
};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    select,
    sync::mpsc,
};

pub async fn handle_client(
    state: &ConnectionSharedState,
    mut response_receiver: mpsc::Receiver<Vec<u8>>,
    mut client_read: impl AsyncRead + Unpin + Send,
    mut srv_write: impl AsyncWrite + Unpin + Send,
) -> Result<(), DecodeError> {
    loop {
        select! {
            msg = response_receiver.recv() => {
                let msg = match msg {
                    Some(v) => v,
                    None => break,
                };

                let _ = write_packet(&mut srv_write, &GameServerBoundPacket::ServerBoundPluginMessage(PlayPluginMessage {
                    channel: "basileia:proxy".into(),
                    data: msg
                })).await.map_err(|error| {
                    tracing::error!(%error, "Failed to send command response to proxied server");
                });
            }
            vec = read_packet(&mut client_read, true) => {
                let vec = match vec? {
                    Some(v) => v,
                    None => break,
                };

                let packet_result = state.decode_client(&vec).await;
                let current_state = state.current_state().await;

                match packet_result {
                    Ok(Some(packet)) => {
                        tracing::trace!(?current_state, ?packet, "Incomming client packet");

                        match packet {
                            ClientPacket::Login(LoginServerBoundPacket::LoginAcknowledged) => {
                                state.set_state(ProtocolState::Configuration).await;
                                tracing::debug!("Entered configuration state");
                            }
                            ClientPacket::Configuration(
                                ConfigServerBoundPacket::AcknowledgeFinishConfiguration,
                            ) => {
                                state.set_state(ProtocolState::Play).await;
                                tracing::debug!("Entered play state");
                            }
                            _ => {}
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            ?current_state,
                            %error,
                            "Incomming client packet could not be decoded"
                        );
                    }
                    _ => {
                        tracing::warn!(
                            ?current_state,
                            "Incomming client packet could not be decoded"
                        );
                    }
                }

                srv_write.write_all(&vec).await?;
            }
        }
    }

    Ok(())
}

pub async fn handle_server(
    global_state: &GlobalSharedState,
    state: &ConnectionSharedState,
    request_sender: mpsc::Sender<Vec<u8>>,
    mut srv_read: impl AsyncRead + Unpin + Send,
    mut client_write: impl AsyncWrite + Unpin + Send,
) -> Result<(), DecodeError> {
    loop {
        let vec = match read_packet(&mut srv_read, true).await? {
            Some(v) => v,
            None => break,
        };

        let packet_result = state.decode_server(&vec).await;
        let current_state = state.current_state().await;

        match packet_result {
            Ok(Some(packet)) => {
                tracing::trace!(?current_state, ?packet, "Incomming server packet");

                match packet {
                    ServerPacket::Login(LoginClientBoundPacket::LoginSuccess(packet)) => {
                        tracing::info!(
                            username = %packet.username,
                            uuid = %packet.uuid,
                            "Login success"
                        );
                        let mut lock = state.login_info.write().await;
                        *lock = Some(PostLoginInformation {
                            username: packet.username.clone(),
                            uuid: packet.uuid,
                        });
                        drop(lock);

                        global_state
                            .add_online_player(packet.username, packet.uuid)
                            .await;
                    }
                    ServerPacket::Login(LoginClientBoundPacket::SetCompression(packet)) => {
                        tracing::debug!(threshold = packet.threshold, "Set compression");
                        if 0 > packet.threshold {
                            break;
                        }
                        state.set_compression(packet.threshold as usize).await;
                    }
                    ServerPacket::Configuration(ConfigClientBoundPaket::FinishConfiguration) => {
                        state.set_state(ProtocolState::Play).await;
                        tracing::debug!("Entered play state");
                    }
                    ServerPacket::Play(GameClientBoundPacket::ClientBoundPluginMessage(
                        plugin_message,
                    )) => {
                        if plugin_message.channel == "basileia:proxy" {
                            if request_sender.send(plugin_message.data).await.is_err() {
                                tracing::error!("Command data sender closed earlier than expected");
                                break;
                            }
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            Err(error) => {
                tracing::warn!(
                    ?current_state,
                    %error,
                    "Incomming server packet could not be decoded"
                );
            }
            _ => {
                tracing::warn!(
                    ?current_state,
                    "Incomming server packet could not be decoded"
                );
            }
        }

        client_write.write_all(&vec).await?;
    }

    Ok(())
}
