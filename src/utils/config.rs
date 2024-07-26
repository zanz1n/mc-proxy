use super::BoxDynError;
use serde::Deserialize;
use std::{fmt::Debug, fs};

pub trait Config
where
    Self: Debug,
    Self: Sized,
    for<'de> Self: Deserialize<'de>,
{
    fn auto() -> Result<Self, BoxDynError> {
        if let Some(config_file) = std::env::var("CONFIG_FILE").ok() {
            tracing::info!(
                target: "service_configuration",
                %config_file,
                "Loading configuration from file",
            );

            Self::from_file(config_file)
        } else {
            tracing::info!(
                target: "service_configuration",
                "Loading configuration from environment variables",
            );

            Self::from_env_var()
        }
    }

    fn from_env_var() -> Result<Self, BoxDynError>;

    fn from_file(config_file: String) -> Result<Self, BoxDynError> {
        let string = fs::read_to_string(config_file)?;

        let json = serde_json::from_str(&string)?;

        Ok(json)
    }
}

impl Config for () {
    fn auto() -> Result<Self, BoxDynError> {
        Ok(())
    }

    fn from_env_var() -> Result<Self, BoxDynError> {
        Ok(())
    }

    fn from_file(_: String) -> Result<Self, BoxDynError> {
        Ok(())
    }
}
