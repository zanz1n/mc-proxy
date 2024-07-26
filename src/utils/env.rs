use super::BoxDynError;
use std::{env::VarError, error::Error, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum EnvError<'a> {
    #[error("Could not find environment variable `{0}`")]
    NotFound(&'a str),
    #[error("Environment variable `{0}` is not unicode")]
    NotUnicode(&'a str),
    #[error("Failed to parse environment variable `{0}`: {1}")]
    ParseError(&'a str, BoxDynError),
}

pub fn get<'a>(key: &'a str) -> Result<String, EnvError<'a>> {
    std::env::var(key).map_err(|error| match error {
        VarError::NotPresent => EnvError::NotFound(key),
        VarError::NotUnicode(_) => EnvError::NotUnicode(key),
    })
}

#[inline]
pub fn get_or(key: &str, default: String) -> String {
    std::env::var(key).unwrap_or(default)
}

pub fn get_parsed<'a, T, E>(key: &'a str) -> Result<T, EnvError<'a>>
where
    T: FromStr<Err = E>,
    E: Error + Send + Sync + 'static,
{
    let s = get(key)?;
    T::from_str(&s).map_err(|error| EnvError::ParseError(key, error.into()))
}

pub fn get_parsed_or<'a, T, E>(key: &'a str, default: T) -> Result<T, EnvError<'a>>
where
    T: FromStr<Err = E> + Sized,
    E: Error + Send + Sync + 'static,
{
    match get(key) {
        Ok(s) => T::from_str(&s).map_err(|error| EnvError::ParseError(key, error.into())),
        Err(error) => match error {
            EnvError::NotFound(_) => Ok(default),
            _ => Err(error),
        },
    }
}
