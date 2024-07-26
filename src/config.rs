use crate::utils::{self, env, BoxDynError};
use minecraft_protocol::data::chat::Message;
use serde::Deserialize;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: SocketAddr,
    pub proxied_addr: String,
    pub sqlite_file: String,
    pub server_status: Message,
}

impl utils::Config for Config {
    fn from_env_var() -> Result<Self, BoxDynError> {
        Ok(Self {
            listen_addr: env::get_parsed_or("LISTEN_ADDR", default_listen_addr())?,
            proxied_addr: env::get("PROXIED_ADDR")?,
            sqlite_file: env::get_or("SQLITE_FILE", "proxy.sqlite".into()),
            server_status: serde_json::from_str(&env::get("SERVER_STATUS")?)?,
        })
    }
}

const fn default_listen_addr() -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 25565))
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn assert_json_config_parses() {
        const CONFIG_FILE: &'static str = include_str!("../config.example.json");

        serde_json::from_str::<'_, Config>(CONFIG_FILE)
            .expect("Failed to parse config.example.json");
    }
}
