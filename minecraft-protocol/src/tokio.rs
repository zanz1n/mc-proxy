use crate::error::DecodeError;
use std::future::Future;
use tokio::io::{AsyncRead, AsyncReadExt};

pub trait AsyncDecoder {
    type Output;

    fn decode_async<R: AsyncRead + Unpin + Send>(
        reader: &mut R,
    ) -> impl Future<Output = Result<Self::Output, DecodeError>> + Send;
}

pub trait AsyncDecoderReadExt: Send {
    fn read_bool_async(&mut self) -> impl Future<Output = Result<bool, DecodeError>> + Send;

    fn read_string_async(
        &mut self,
        max_length: u16,
    ) -> impl Future<Output = Result<String, DecodeError>> + Send;

    fn read_byte_array_async(
        &mut self,
    ) -> impl Future<Output = Result<Vec<u8>, DecodeError>> + Send;

    fn read_var_i32_async(&mut self) -> impl Future<Output = Result<i32, DecodeError>> + Send;

    fn read_var_i64_async(&mut self) -> impl Future<Output = Result<i64, DecodeError>> + Send;
}

macro_rules! read_signed_var_int (
    ($type: ident, $name: ident, $max_bytes: expr) => (
        async fn $name(&mut self) -> Result<$type, DecodeError> {
            let mut num_read = 0;
            let mut result: $type = 0;

            loop {
                let read = self.read_u8().await?;
                let value = $type::from(read & 0b0111_1111);
                result |= value.overflowing_shl(7 * num_read).0;

                num_read += 1;

                if num_read > 5 {
                    return Err(DecodeError::VarIntTooLong { max_bytes: $max_bytes });
                }
                if read & 0b1000_0000 == 0 {
                    break;
                }
            }
            Ok(result)
        }
   );
);

impl<R: AsyncRead + Unpin + Send> AsyncDecoderReadExt for R {
    async fn read_bool_async(&mut self) -> Result<bool, DecodeError> {
        match self.read_u8().await? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::NonBoolValue),
        }
    }

    async fn read_string_async(&mut self, max_length: u16) -> Result<String, DecodeError> {
        let length = self.read_var_i32_async().await? as usize;

        if length as u16 > max_length {
            return Err(DecodeError::StringTooLong { length, max_length });
        }

        let mut buf = vec![0; length as usize];
        self.read_exact(&mut buf).await?;

        Ok(String::from_utf8(buf)?)
    }

    async fn read_byte_array_async(&mut self) -> Result<Vec<u8>, DecodeError> {
        let length = self.read_var_i32_async().await?;

        let mut buf = vec![0; length as usize];
        self.read_exact(&mut buf).await?;

        Ok(buf)
    }

    read_signed_var_int!(i32, read_var_i32_async, 5);
    read_signed_var_int!(i64, read_var_i64_async, 10);
}
