use crate::{
    decoder::{var_int as var_int_decoder, Decoder},
    encoder::{var_int as var_int_encoder, Encoder},
    error::{DecodeError, EncodeError},
};
use aes::{cipher::KeyIvInit, Aes128};
use bytes::BytesMut;
use cfb8::{cipher::AsyncStreamCipher, Decryptor, Encryptor};
use flate2::{
    read::{ZlibDecoder, ZlibEncoder},
    Compression,
};
use std::io::{Cursor, Read};

pub type CryptKey = [u8; 16];

#[derive(Default)]
pub struct MinecraftCodec {
    crypt_key: Option<CryptKey>,

    compression: Option<usize>,

    received_buf: BytesMut,
    staging_buf: Vec<u8>,

    compression_target: Vec<u8>,
}

impl MinecraftCodec {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn enable_encryption(&mut self, key: CryptKey) {
        self.crypt_key = Some(key);
    }

    #[inline]
    pub fn enable_compression(&mut self, threshold: usize) {
        self.compression = Some(threshold);
    }

    #[inline]
    pub fn clone_with_settings(&self) -> Self {
        Self {
            crypt_key: self.crypt_key,
            compression: self.compression,
            received_buf: BytesMut::new(),
            staging_buf: Vec::new(),
            compression_target: Vec::new(),
        }
    }

    pub fn encode(
        &mut self,
        packet: &impl Encoder,
        output: &mut Vec<u8>,
    ) -> Result<(), EncodeError> {
        packet.encode(&mut self.staging_buf)?;

        if let Some(threshold) = self.compression {
            self.encode_compressed(output, threshold)?;
        } else {
            self.encode_uncompressed(output)?;
        }

        if let Some(key) = &self.crypt_key {
            Encryptor::<Aes128>::new_from_slices(key, key)
                .expect("key size is invalid")
                .encrypt(output)
        }

        self.staging_buf.clear();

        Ok(())
    }

    fn encode_compressed(
        &mut self,
        output: &mut Vec<u8>,
        threshold: usize,
    ) -> Result<(), EncodeError> {
        let (data_length, data) = if self.staging_buf.len() >= threshold {
            self.data_compressed()
        } else {
            self.data_uncompressed()
        };

        const MAX_VAR_INT_LENGTH: usize = 5;
        let mut buf = [0u8; MAX_VAR_INT_LENGTH];
        let data_length_bytes = Cursor::new(&mut buf[..]);
        var_int_encoder::encode(&(data_length as i32), output)?;

        let packet_length = data_length_bytes.position() as usize + data.len();
        var_int_encoder::encode(&(packet_length as i32), output)?;
        var_int_encoder::encode(&(data_length as i32), output)?;

        output.extend_from_slice(data);

        self.compression_target.clear();

        Ok(())
    }

    fn data_compressed(&mut self) -> (usize, &[u8]) {
        let mut encoder = ZlibEncoder::new(self.staging_buf.as_slice(), Compression::default());
        encoder
            .read_to_end(&mut self.compression_target)
            .expect("compression failed");

        (self.staging_buf.len(), self.compression_target.as_slice())
    }

    #[inline]
    fn data_uncompressed(&mut self) -> (usize, &[u8]) {
        (0, self.staging_buf.as_slice())
    }

    fn encode_uncompressed(&mut self, output: &mut Vec<u8>) -> Result<(), EncodeError> {
        let length = self.staging_buf.len() as i32;
        var_int_encoder::encode(&length, output)?;
        output.extend_from_slice(&self.staging_buf);

        Ok(())
    }

    pub fn accept(&mut self, bytes: &[u8]) {
        let start_index = self.received_buf.len();
        self.received_buf.extend(bytes);

        if let Some(key) = &self.crypt_key {
            Decryptor::<Aes128>::new_from_slices(key, key)
                .expect("key size is invalid")
                .decrypt(&mut self.received_buf[start_index..]);
        }
    }

    pub fn next_packet<T>(&mut self) -> Result<Option<T::Output>, DecodeError>
    where
        T: Decoder,
    {
        let mut cursor = Cursor::new(&self.received_buf[..]);
        let packet = if let Ok(length) = var_int_decoder::decode(&mut cursor) {
            let length_field_length = cursor.position() as usize;

            if self.received_buf.len() - length_field_length >= length as usize {
                cursor = Cursor::new(
                    &self.received_buf[length_field_length..length_field_length + length as usize],
                );

                if self.compression.is_some() {
                    let data_length = var_int_decoder::decode(&mut cursor)?;
                    if data_length != 0 {
                        let mut decoder =
                            ZlibDecoder::new(&cursor.get_ref()[cursor.position() as usize..]);
                        decoder.read_to_end(&mut self.compression_target)?;
                        cursor = Cursor::new(&self.compression_target);
                    }
                }

                let packet = T::decode(&mut cursor)?;

                let bytes_read = length as usize + length_field_length;
                self.received_buf = self.received_buf.split_off(bytes_read);

                self.compression_target.clear();
                Some(packet)
            } else {
                None
            }
        } else {
            None
        };

        Ok(packet)
    }
}
