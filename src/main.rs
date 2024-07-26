use crate::{config::Config, state::GlobalSharedState, utils::touch_file};
use repository::{
    ip_bans::SqlxIpBansRepository, kv::SqlxKeyValueRepository, user_bans::SqlxUserBansRepository,
    whitelist::SqlxWhitelistRepository,
};
use server::Server;
use sqlx::{migrate, SqlitePool};
use std::{io::Error, sync::Arc, time::Instant};
use tokio::net::TcpListener;
use tracing::{Instrument, Level};
use utils::{
    service::{config_and_init_service, graceful_shutdown},
    BoxDynError,
};

mod commands;
mod config;
mod errors;
mod handler;
mod repository;
mod server;
mod state;
mod utils;

async fn listen_loop(listener: TcpListener, srv: Arc<Server>) -> Error {
    loop {
        let (conn, address) = match listener.accept().await {
            Ok(v) => v,
            Err(err) => return err,
        };

        let srv = srv.clone();
        tokio::task::spawn(async move {
            let _ = srv
                .handle_conn(conn, address)
                .instrument(tracing::span!(Level::ERROR, "connection", %address))
                .await;
        });
    }
}

async fn run_service(config: Config) -> Result<(), BoxDynError> {
    touch_file(&config.sqlite_file).await?;

    let listener = TcpListener::bind(config.listen_addr).await?;
    tracing::info!(
        port = config.listen_addr.port(),
        "Listening for connections"
    );

    let pool = SqlitePool::connect(&format!("sqlite:{}", config.sqlite_file)).await?;

    let migration_start = Instant::now();
    migrate!().run(&pool).await?;

    tracing::info!(
        took = ?(Instant::now() - migration_start),
        file_path = config.sqlite_file,
        "Migrations were run on sqlite",
    );

    let key_value = SqlxKeyValueRepository::new(pool.clone());

    let ip_bans = SqlxIpBansRepository::new(pool.clone());
    let user_bans = SqlxUserBansRepository::new(pool.clone());

    let global_state = GlobalSharedState::new(
        config.server_status,
        ip_bans,
        user_bans,
        SqlxWhitelistRepository::new(pool.clone(), key_value),
    );

    let srv = Arc::new(Server::new(config.proxied_addr, global_state));
    let tcp_end = tokio::spawn(listen_loop(listener, srv));

    graceful_shutdown(tcp_end).await?;
    tracing::info!("Shutting down service ...");
    pool.close().await;

    Ok(())
}

fn main() {
    config_and_init_service(run_service)
}
