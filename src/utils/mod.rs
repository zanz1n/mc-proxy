use minecraft_protocol::{
    encoder::{var_int, Encoder},
    error::{DecodeError, EncodeError},
    tokio::AsyncDecoderReadExt,
};
use std::{
    error::Error,
    io::{self, ErrorKind},
};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

pub type BoxDynError = Box<dyn Error + Send + Sync>;

pub mod config;
pub mod env;
pub mod service;

pub use config::Config;

pub fn encode_packet<T: Encoder>(data: &T) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::new();

    data.encode(&mut buf).unwrap();
    let mut vec = Vec::new();

    var_int::encode(&(buf.len() as i32), &mut vec).unwrap();
    vec.extend(buf);

    Ok(vec)
}

pub async fn write_packet<W: AsyncWrite + Unpin + Send, T: Encoder>(
    writer: &mut W,
    data: &T,
) -> Result<(), io::Error> {
    let vec = encode_packet(data).unwrap();

    writer.write_all(&vec).await?;
    Ok(())
}

pub async fn read_packet<R: AsyncRead + Unpin + Send>(
    reader: &mut R,
    encode_length: bool,
) -> Result<Option<Vec<u8>>, DecodeError> {
    let length = reader.read_var_i32_async().await?;
    if length == 0 || 0 > length {
        return Ok(None);
    }

    let mut buf = vec![0; length as usize];

    reader.read_exact(&mut buf).await?;
    if encode_length {
        let mut vec = Vec::new();
        var_int::encode(&length, &mut vec).unwrap();

        vec.extend(buf);
        if vec.is_empty() {
            return Ok(None);
        }

        Ok(Some(vec))
    } else {
        Ok(Some(buf))
    }
}

pub async fn touch_file(path: &str) -> io::Result<()> {
    let file = File::open(path).await;

    if let Err(err) = file {
        if err.kind() == ErrorKind::NotFound {
            File::create(path).await?;
            Ok(())
        } else {
            Err(err)
        }
    } else {
        Ok(())
    }
}
