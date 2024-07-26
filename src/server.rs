use crate::{
    commands::handler::proxy_command_events,
    errors::AppError,
    handler::{
        handshake::handle_handshake,
        login::handle_login_start,
        proxy::{handle_client, handle_server},
        status::handle_status,
    },
    repository::ip_bans::IpBansRepository,
    state::{ConnectionSharedState, GlobalSharedState},
    utils::write_packet,
};
use minecraft_protocol::{
    codec::ProtocolState,
    packet::{
        handshake::{Handshake, HandshakeServerBoundPacket, NextState},
        login::{LoginClientBoundPacket, LoginDisconnect, LoginServerBoundPacket, LoginStart},
    },
};
use std::{
    io::{self},
    net::SocketAddr,
};
use tokio::{
    net::{lookup_host, TcpStream},
    sync::mpsc,
};

pub struct Server {
    proxied_address: String,
    global_state: GlobalSharedState,
}

impl Server {
    pub fn new(addr: String, global_state: GlobalSharedState) -> Self {
        Self {
            proxied_address: addr,
            global_state,
        }
    }

    pub async fn handle_conn(
        &self,
        mut incomming: TcpStream,
        address: SocketAddr,
    ) -> Result<(), AppError> {
        let ban = self.global_state.ip_bans.is_banned(address.ip()).await?;

        if let Some(ban) = ban {
            tracing::info!(
                reason = ban.reason,
                banned_at = ?ban.created_at,
                banned_until = ?ban.expiration,
                "Connection rejected: IP banned",
            );

            return Ok(());
        }

        tracing::info!("Incomming connection");

        let handshake = match handle_handshake(&mut incomming).await {
            Ok(v) => v,
            Err(error) => {
                tracing::warn!(%error, "Client didn't send handshake properly");
                return Ok(());
            }
        };

        tracing::debug!(
            protocol = handshake.protocol_version,
            next_state = ?handshake.next_state,
            "Connection finished handshake",
        );

        tracing::info!("Connection is of {:?} type", handshake.next_state);

        match handshake.next_state {
            NextState::Status => {
                let _ = handle_status(&self.global_state, &handshake, &mut incomming)
                    .await
                    .map_err(|error| {
                        if !error.is_eof_error() {
                            tracing::warn!(%error, "Client error on status connection");
                        }
                    });

                tracing::info!(
                    protocol = handshake.protocol_version,
                    "Status connection closed"
                );
            }
            NextState::Login => {
                if !self.check_protocol_version(handshake.protocol_version) {
                    let _ = write_packet(
                        &mut incomming,
                        &LoginClientBoundPacket::LoginDisconnect(LoginDisconnect {
                            reason: r#"{"text":"Your minecraft version is not accepted"}"#.into(),
                        }),
                    )
                    .await
                    .map_err(|error| {
                        tracing::warn!(%error, "Failed to send login disconnect message");
                    });

                    tracing::info!(
                        protocol = handshake.protocol_version,
                        "Connection closed: invalid protocol version"
                    );
                } else {
                    let login_start =
                        match handle_login_start(&self.global_state, &mut incomming).await {
                            Ok(Some(v)) => v,
                            _ => {
                                tracing::info!(
                                    protocol = handshake.protocol_version,
                                    "Connection closed during login start",
                                );
                                return Ok(());
                            }
                        };

                    self.handle_proxy(incomming, login_start, handshake).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn handle_proxy(
        &self,
        mut incomming: TcpStream,
        login_start: LoginStart,
        handshake: Handshake,
    ) -> Result<(), AppError> {
        let mut srv = self.connect_to_server().await?;

        let result1 = write_packet(
            &mut srv,
            &HandshakeServerBoundPacket::Handshake(handshake.clone()),
        )
        .await
        .map_err(|error| {
            tracing::error!(%error, "Failed to send handshake packet to proxied server");
        });

        let result2 = write_packet(&mut srv, &LoginServerBoundPacket::LoginStart(login_start))
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to send login start packt to proxied server");
            });

        if result1.is_err() || result2.is_err() {
            tracing::info!(protocol = handshake.protocol_version, "Connection closed");
            return Ok(());
        }

        let (srv_read, srv_write) = srv.split();
        let (client_read, client_write) = incomming.split();

        let state = ConnectionSharedState::new(handshake.protocol_version);
        state.set_state(ProtocolState::Login).await;

        let (request_sender, request_receiver) = mpsc::channel(3);
        let (response_sender, response_receiver) = mpsc::channel(3);

        tokio::select! {
            r = handle_server(&self.global_state, &state, request_sender, srv_read, client_write) => {
                if let Err(error) = r {
                    if !error.is_eof_error() {
                        tracing::warn!(%error, "Server error");
                    }
                }
            }
            r = handle_client(&state, response_receiver, client_read, srv_write) => {
                if let Err(error) = r {
                    if !error.is_eof_error() {
                        tracing::warn!(%error, "Client error");
                    }
                }
            }
            _ = proxy_command_events(&self.global_state, request_receiver, response_sender) => {}
        }

        match state.login_username().await {
            Some(username) => {
                self.global_state.remove_online_player(&username).await;
                tracing::info!(
                    username,
                    protocol = state.protocol_version,
                    "Connection closed"
                );
            }
            None => {
                tracing::info!(protocol = state.protocol_version, "Connection closed");
            }
        }

        Ok(())
    }

    fn check_protocol_version(&self, protocol_version: i32) -> bool {
        protocol_version == 765
    }

    async fn resolve_dns(&self) -> Result<SocketAddr, io::Error> {
        lookup_host(&self.proxied_address)
            .await?
            .next()
            .ok_or(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "Failed to resolve proxied server address",
            ))
    }

    async fn connect_to_server(&self) -> Result<TcpStream, io::Error> {
        let host = self.resolve_dns().await.map_err(|error| {
            tracing::error!(%error, "Failed to resolve proxied server address");
            error
        })?;

        TcpStream::connect(host).await.map_err(|error| {
            tracing::error!(%error, "Failed to connect to proxied server");
            error
        })
    }
}
