use super::{BoxDynError, Config};
use std::future::Future;
use tokio::runtime::Builder;
use tracing_subscriber::EnvFilter;

pub fn config_and_init_service<C, F, Fut>(service_fn: F)
where
    C: Config,
    F: Fn(C) -> Fut,
    Fut: Future<Output = Result<(), BoxDynError>>,
{
    #[cfg(feature = "dotenv")]
    {
        if let Err(e) = dotenvy::dotenv() {
            eprintln!("Failed to load environment variables from .env file: {e}");
        }
    }

    #[cfg(not(feature = "json-log"))]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    #[cfg(feature = "json-log")]
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = match C::auto() {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(
                target: "service_configuration",
                %error,
                "Failed to load configuration",
            );
            eprintln!("Failed to load service configuration: {error}");
            std::process::exit(1);
        }
    };

    tracing::info!(target: "service_configuration", ?config, "Loaded configuration");

    let async_rt_result = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(service_fn(config));

    if let Err(e) = async_rt_result {
        eprintln!("Unhandled fatal error: {e}");
        std::process::exit(1);
    }
}

#[cfg(unix)]
pub fn shutdown_signal() -> std::io::Result<impl Future<Output = ()>> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;

    Ok(async move {
        tokio::select! {
            _ = interrupt.recv() => {
                tracing::info!(target: "service_signals", "Received SIGING");
            }
            _ = terminate.recv() => {
                tracing::info!(target: "service_signals", "Received SIGTERM");
            }
        }
    })
}

#[cfg(not(unix))]
pub fn shutdown_signal() -> std::io::Result<impl Future<Output = ()>> {
    use futures_util::FutureExt;

    Ok(tokio::signal::ctrl_c().map(|res| match res {
        Ok(_) => {
            tracing::info!(target: "service_signals", "Received CTRL_C");
        }
        Err(_) => {
            tracing::error!(target: "service_signals", "Failed to create CTRL_C signal receiver");
        }
    }))
}

pub async fn graceful_shutdown(task: impl std::future::Future) -> std::io::Result<()> {
    let signal = shutdown_signal()?;

    tokio::select! {
        _ = signal => {}
        _ = task => {
            tracing::info!(target: "service_signals", "Service main task exited");
        }
    }

    Ok(())
}
