use crate::nbt::CompoundTag;
use crate::{error::DecodeError, nbt::decode::read_compound_tag};
use byteorder::{BigEndian, ReadBytesExt};
use std::io::Read;
use uuid::Uuid;

pub trait Decoder {
    type Output;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError>;
}

pub trait EnumDecoder {
    type Output;

    fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self::Output, DecodeError>;
}

impl<T: EnumDecoder> Decoder for T {
    type Output = T::Output;

    #[inline]
    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        let type_id = var_int::decode(reader)?
            .try_into()
            .map_err(|_| DecodeError::VarIntTooLong { max_bytes: 1 })?;

        <T as EnumDecoder>::decode(type_id, reader)
    }
}

/// Trait adds additional helper methods for `Read` to read protocol data.
pub trait DecoderReadExt {
    fn read_bool(&mut self) -> Result<bool, DecodeError>;

    fn read_string(&mut self, max_length: u16) -> Result<String, DecodeError>;

    fn read_byte_array(&mut self) -> Result<Vec<u8>, DecodeError>;

    fn read_compound_tag(&mut self) -> Result<CompoundTag, DecodeError>;

    fn read_var_i32(&mut self) -> Result<i32, DecodeError>;

    fn read_var_i64(&mut self) -> Result<i64, DecodeError>;
}

macro_rules! read_signed_var_int (
    ($type: ident, $name: ident, $max_bytes: expr) => (
        fn $name(&mut self) -> Result<$type, DecodeError> {
            let mut num_read = 0;
            let mut result: $type = 0;

            loop {
                let read = self.read_u8()?;
                let value = $type::from(read & 0b0111_1111);
                result |= value.overflowing_shl(7 * num_read).0;

                num_read += 1;

                if num_read > $max_bytes {
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

impl<R: Read> DecoderReadExt for R {
    fn read_bool(&mut self) -> Result<bool, DecodeError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::NonBoolValue),
        }
    }

    fn read_string(&mut self, max_length: u16) -> Result<String, DecodeError> {
        let length = self.read_var_i32()? as usize;

        if length > max_length as usize {
            return Err(DecodeError::StringTooLong { length, max_length });
        }

        let mut buf = vec![0; length];
        self.read_exact(&mut buf)?;

        Ok(String::from_utf8(buf)?)
    }

    fn read_byte_array(&mut self) -> Result<Vec<u8>, DecodeError> {
        let length = self.read_var_i32()? as usize;

        let mut buf = vec![0; length];
        self.read_exact(&mut buf)?;

        Ok(buf)
    }

    fn read_compound_tag(&mut self) -> Result<CompoundTag, DecodeError> {
        Ok(read_compound_tag(self)?)
    }

    read_signed_var_int!(i32, read_var_i32, 5);
    read_signed_var_int!(i64, read_var_i64, 10);
}

impl Decoder for u8 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_u8()?)
    }
}

impl Decoder for i16 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_i16::<BigEndian>()?)
    }
}

impl Decoder for i32 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_i32::<BigEndian>()?)
    }
}

impl Decoder for u16 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_u16::<BigEndian>()?)
    }
}

impl Decoder for u32 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_u32::<BigEndian>()?)
    }
}

impl Decoder for i64 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_i64::<BigEndian>()?)
    }
}

impl Decoder for u64 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_u64::<BigEndian>()?)
    }
}

impl Decoder for f32 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_f32::<BigEndian>()?)
    }
}

impl Decoder for f64 {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_f64::<BigEndian>()?)
    }
}

impl Decoder for String {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_string(32_768)?)
    }
}

impl Decoder for bool {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_bool()?)
    }
}

impl Decoder for Vec<u8> {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_byte_array()?)
    }
}

impl Decoder for Uuid {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        let mut buf = [0; 16];
        reader.read_exact(&mut buf)?;

        Ok(Uuid::from_bytes(buf))
    }
}

impl Decoder for CompoundTag {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        Ok(reader.read_compound_tag()?)
    }
}

impl Decoder for Vec<CompoundTag> {
    type Output = Self;

    fn decode<R: Read>(reader: &mut R) -> Result<Self::Output, DecodeError> {
        let length = reader.read_var_i32()? as usize;
        let mut vec = Vec::with_capacity(length);

        for _ in 0..length {
            let compound_tag = reader.read_compound_tag()?;
            vec.push(compound_tag);
        }

        Ok(vec)
    }
}

pub mod var_int {
    use crate::decoder::DecoderReadExt;
    use crate::error::DecodeError;
    use std::io::Read;

    pub fn decode<R: Read>(reader: &mut R) -> Result<i32, DecodeError> {
        Ok(reader.read_var_i32()?)
    }
}

pub mod var_long {
    use crate::decoder::DecoderReadExt;
    use crate::error::DecodeError;
    use std::io::Read;

    pub fn decode<R: Read>(reader: &mut R) -> Result<i64, DecodeError> {
        Ok(reader.read_var_i64()?)
    }
}

pub mod rest {
    use crate::error::DecodeError;
    use std::io::Read;

    pub fn decode<R: Read>(reader: &mut R) -> Result<Vec<u8>, DecodeError> {
        let mut data = Vec::new();
        reader.read_to_end(data.as_mut())?;

        Ok(data)
    }
}

pub mod uuid_hyp_str {
    use crate::decoder::DecoderReadExt;
    use crate::error::DecodeError;
    use std::io::Read;
    use uuid::Uuid;

    pub fn decode<R: Read>(reader: &mut R) -> Result<Uuid, DecodeError> {
        let uuid_hyphenated_string = reader.read_string(36)?;
        let uuid = Uuid::parse_str(&uuid_hyphenated_string)?;

        Ok(uuid)
    }
}

pub mod bool_option {
    use crate::decoder::{Decoder, DecoderReadExt};
    use crate::error::DecodeError;
    use std::io::Read;

    pub fn decode<R: Read, T: Decoder<Output = T>>(
        reader: &mut R,
    ) -> Result<Option<T>, DecodeError> {
        let bool = reader.read_bool()?;

        match bool {
            true => {
                let data = T::decode(reader)?;
                Ok(Some(data))
            }
            false => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::decoder::DecoderReadExt;
    use std::io::Cursor;

    #[test]
    fn test_read_variable_i32_2_bytes_value() {
        let mut cursor = Cursor::new(vec![0b10101100, 0b00000010]);
        let value = cursor.read_var_i32().unwrap();

        assert_eq!(value, 300);
    }

    #[test]
    fn test_read_variable_i32_5_bytes_value() {
        let mut cursor = Cursor::new(vec![0xff, 0xff, 0xff, 0xff, 0x07]);
        let value = cursor.read_var_i32().unwrap();

        assert_eq!(value, 2147483647);
    }
}
