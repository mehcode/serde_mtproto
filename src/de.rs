//! Deserialize MTProto binary representation to a Rust data structure.

use std::io;

use byteorder::{ReadBytesExt, LittleEndian};
use serde::de::{self, Deserialize, DeserializeOwned, DeserializeSeed, Visitor};

use error::{self, DeErrorKind, DeSerdeType};
use identifiable::{BOOL_FALSE_ID, BOOL_TRUE_ID};
use utils::{safe_float_cast, safe_int_cast};


/// A structure that deserializes  MTProto binary representation into Rust values.
#[derive(Debug)]
pub struct Deserializer<R: io::Read> {
    reader: R,
    enum_variant_id: Option<&'static str>,
}

impl<R: io::Read> Deserializer<R> {
    /// Create a MTProto deserializer from an `io::Read` and enum variant hint.
    pub fn new(reader: R, enum_variant_id: Option<&'static str>) -> Deserializer<R> {
        Deserializer {
            reader: reader,
            enum_variant_id: enum_variant_id,
        }
    }

    /// Unwraps the `Deserializer` and returns the underlying `io::Read`.
    pub fn into_reader(self) -> R {
        self.reader
    }

    /// Consumes the `Deserializer` and returns remaining unprocessed bytes.
    pub fn remaining_bytes(mut self) -> error::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.reader.read_to_end(&mut buf)?;

        Ok(buf)
    }

    fn get_str_info(&mut self) -> error::Result<(usize, usize)> {
        let first_byte = self.reader.read_u8()?;
        let len;
        let rem;

        if first_byte <= 253 {
            len = usize::from(first_byte);
            rem = (len + 1) % 4;
        } else if first_byte == 254 {
            let uncasted = self.reader.read_u24::<LittleEndian>()?;
            len = safe_int_cast::<u32, usize>(uncasted)?;
            rem = len % 4;
        } else { // must be 255
            assert_eq!(first_byte, 255);
            return Err(de::Error::invalid_value(
                de::Unexpected::Unsigned(255),
                &"a byte in [0..254] range"));
        }

        let padding = (4 - rem) % 4;

        Ok((len, padding))
    }

    fn read_string(&mut self) -> error::Result<String> {
        let s_bytes = self.read_byte_buf()?;
        let s = String::from_utf8(s_bytes)?;

        Ok(s)
    }

    fn read_byte_buf(&mut self) -> error::Result<Vec<u8>> {
        let (len, padding) = self.get_str_info()?;

        let mut b = vec![0; len];
        self.reader.read_exact(&mut b)?;

        let mut p = vec![0; padding];
        self.reader.read_exact(&mut p)?;

        Ok(b)
    }
}

impl<'a> Deserializer<&'a [u8]> {
    /// Length of unprocessed data in the byte buffer.
    pub fn remaining_length(&self) -> usize {
        self.reader.len()
    }
}


macro_rules! impl_deserialize_small_int {
    ($small_type:ty, $small_deserialize:ident, $big_read:ident::<$big_endianness:ident>,
     $small_visit:ident
    ) => {
        fn $small_deserialize<V>(self, visitor: V) -> error::Result<V::Value>
            where V: Visitor<'de>
        {
            let value = self.reader.$big_read::<$big_endianness>()?;
            debug!("Deserialized big int: {:#x}", value);
            let casted = safe_int_cast(value)?;
            debug!("Casted to {}: {:#x}", stringify!($small_type), casted);

            visitor.$small_visit(casted)
        }
    };
}

macro_rules! impl_deserialize_big_int {
    ($type:ty, $deserialize:ident, $read:ident::<$endianness:ident>, $visit:ident) => {
        fn $deserialize<V>(self, visitor: V) -> error::Result<V::Value>
            where V: Visitor<'de>
        {
            let value = self.reader.$read::<$endianness>()?;
            debug!("Deserialized {}: {:#x}", stringify!($type), value);

            visitor.$visit(value)
        }
    };
}

impl<'de, 'a, R> de::Deserializer<'de> for &'a mut Deserializer<R>
    where R: io::Read
{
    type Error = error::Error;

    fn deserialize_any<V>(self, _visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Any));
    }

    fn deserialize_bool<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let id_value = self.reader.read_u32::<LittleEndian>()?;

        let value = match id_value {
            BOOL_FALSE_ID => false,
            BOOL_TRUE_ID => true,
            _ => {
                return Err(de::Error::invalid_value(
                    de::Unexpected::Signed(i64::from(id_value)),
                    &format!("either {} for false or {} for true", BOOL_FALSE_ID, BOOL_TRUE_ID).as_str()));
            }
        };

        debug!("Deserialized bool: {}", value);

        visitor.visit_bool(value)
    }

    impl_deserialize_small_int!(i8,  deserialize_i8,  read_i32::<LittleEndian>, visit_i8);
    impl_deserialize_small_int!(i16, deserialize_i16, read_i32::<LittleEndian>, visit_i16);
    impl_deserialize_big_int!(i32, deserialize_i32, read_i32::<LittleEndian>, visit_i32);
    impl_deserialize_big_int!(i64, deserialize_i64, read_i64::<LittleEndian>, visit_i64);

    impl_deserialize_small_int!(u8,  deserialize_u8,  read_u32::<LittleEndian>, visit_u8);
    impl_deserialize_small_int!(u16, deserialize_u16, read_u32::<LittleEndian>, visit_u16);
    impl_deserialize_big_int!(u32, deserialize_u32, read_u32::<LittleEndian>, visit_u32);
    impl_deserialize_big_int!(u64, deserialize_u64, read_u64::<LittleEndian>, visit_u64);

    fn deserialize_f32<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let value = self.reader.read_f64::<LittleEndian>()?;
        debug!("Deserialized big float: {}", value);

        let casted = safe_float_cast(value)?;
        debug!("Casted to f32: {}", casted);

        visitor.visit_f32(casted)
    }

    fn deserialize_f64<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let value = self.reader.read_f64::<LittleEndian>()?;
        debug!("Deserialized f64: {}", value);

        visitor.visit_f64(value)
    }

    fn deserialize_char<V>(self, _visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Char));
    }

    fn deserialize_str<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let s = self.read_string()?;
        debug!("Deserialized str: {:?}", s);
        visitor.visit_str(&s)
    }

    fn deserialize_string<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let s = self.read_string()?;
        debug!("Deserialized string: {:?}", s);
        visitor.visit_string(s)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let b = self.read_byte_buf()?;
        debug!("Deserialized bytes: {:?}", b);
        visitor.visit_bytes(&b)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let b = self.read_byte_buf()?;
        debug!("Deserialized byte buffer: {:?}", b);
        visitor.visit_byte_buf(b)
    }

    fn deserialize_option<V>(self, _visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Option));
    }

    fn deserialize_unit<V>(self, _visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::Unit));
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserialized unit struct {}", name);
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing newtype struct {}", name);
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let len = self.reader.read_u32::<LittleEndian>()?;
        debug!("Deserializing seq of len {}", len);

        visitor.visit_seq(SeqAccess::new(self, len))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing tuple of len {}", len);
        visitor.visit_seq(SeqAccess::new(self, safe_int_cast(len)?))
    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing tuple struct {} of len {}", name, len);
        visitor.visit_seq(SeqAccess::new(self, safe_int_cast(len)?))
    }

    fn deserialize_map<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        let len = self.reader.read_u32::<LittleEndian>()?;
        debug!("Deserializing map of len {}", len);

        visitor.visit_map(MapAccess::new(self, len))
    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing struct {} with fields {:?}", name, fields);
        visitor.visit_seq(SeqAccess::new(self, safe_int_cast(fields.len())?))
    }

    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing enum {} with variants {:?}", name, variants);
        visitor.visit_enum(EnumVariantAccess::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing identifier");
        let variant_id = self.enum_variant_id.unwrap();
        debug!("Deserialized variant_id {}", variant_id);

        visitor.visit_str(variant_id)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        bail!(DeErrorKind::UnsupportedSerdeType(DeSerdeType::IgnoredAny));
    }
}


#[derive(Debug)]
struct SeqAccess<'a, R: 'a + io::Read> {
    de: &'a mut Deserializer<R>,
    len: u32,
    next_index: u32,
}

impl<'a, R: io::Read> SeqAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>, len: u32) -> SeqAccess<'a, R> {
        SeqAccess {
            de: de,
            next_index: 0,
            len: len,
        }
    }
}

impl<'de, 'a, R> de::SeqAccess<'de> for SeqAccess<'a, R>
    where R: 'a + io::Read
{
    type Error = error::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> error::Result<Option<T::Value>>
        where T: DeserializeSeed<'de>
    {
        if self.next_index < self.len {
            self.next_index += 1;
        } else {
            debug!("SeqAccess::next_element_seed() is called when no elements is left to deserialize");
            return Ok(None);
        }

        debug!("Deserializing sequence element");
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        safe_int_cast(self.len - self.next_index).ok()
    }
}


#[derive(Debug)]
struct MapAccess<'a, R: 'a + io::Read> {
    de: &'a mut Deserializer<R>,
    len: u32,
    next_index: u32,
}

impl<'a, R: io::Read> MapAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>, len: u32) -> MapAccess<'a, R> {
        MapAccess {
            de: de,
            next_index: 0,
            len: len,
        }
    }
}

impl<'de, 'a, R> de::MapAccess<'de> for MapAccess<'a, R>
    where R: 'a + io::Read
{
    type Error = error::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> error::Result<Option<K::Value>>
        where K: DeserializeSeed<'de>
    {
        if self.next_index < self.len {
            self.next_index += 1;
        } else {
            debug!("MapAccess::next_element_seed() is called when no elements is left to deserialize");
            return Ok(None);
        }

        debug!("Deserializing map key");
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> error::Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        debug!("Deserializing map value");
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        safe_int_cast(self.len - self.next_index).ok()
    }
}


#[derive(Debug)]
struct EnumVariantAccess<'a, R: 'a + io::Read> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R: io::Read> EnumVariantAccess<'a, R> {
    fn new(de: &'a mut Deserializer<R>) -> EnumVariantAccess<'a, R> {
        EnumVariantAccess { de: de }
    }
}

impl<'de, 'a, R> de::EnumAccess<'de> for EnumVariantAccess<'a, R>
    where R: 'a + io::Read
{
    type Error = error::Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> error::Result<(V::Value, Self::Variant)>
        where V: DeserializeSeed<'de>
    {
        debug!("Deserializing enum variant");
        let value = seed.deserialize(&mut *self.de)?;

        Ok((value, self))
    }
}

impl<'de, 'a, R> de::VariantAccess<'de> for EnumVariantAccess<'a, R>
    where R: 'a + io::Read
{
    type Error = error::Error;

    fn unit_variant(self) -> error::Result<()> {
        debug!("Deserialized unit variant");
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> error::Result<T::Value>
        where T: DeserializeSeed<'de>
    {
        debug!("Deserializing newtype variant");
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing tuple variant");
        de::Deserializer::deserialize_tuple_struct(self.de, "", len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> error::Result<V::Value>
        where V: Visitor<'de>
    {
        debug!("Deserializing struct variant");
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}


/// Deserialize an instance of type `T` from bytes of binary MTProto.
pub fn from_bytes<'a, T>(bytes: &'a [u8], enum_variant_id: Option<&'static str>) -> error::Result<T>
    where T: Deserialize<'a>
{
    let mut de = Deserializer::new(bytes, enum_variant_id);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok(value)
}

/// Deserialize an instance of type `T` from bytes of binary MTProto and return unused bytes.
pub fn from_bytes_reuse<'a, T>(bytes: &'a [u8],
                               enum_variant_id: Option<&'static str>)
                              -> error::Result<(T, &'a [u8])>
    where T: Deserialize<'a>
{
    let mut de = Deserializer::new(bytes, enum_variant_id);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok((value, de.reader))
}

/// Deserialize an instance of type `T` from an IO stream of binary MTProto.
pub fn from_reader<R, T>(reader: R, enum_variant_id: Option<&'static str>) -> error::Result<T>
    where R: io::Read,
          T: DeserializeOwned,
{
    let mut de = Deserializer::new(reader, enum_variant_id);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok(value)
}

/// Deserialize an instance of type `T` from an IO stream of binary MTProto and return unused part
/// of IO stream.
pub fn from_reader_reuse<R, T>(reader: R,
                               enum_variant_id: Option<&'static str>)
                              -> error::Result<(T, R)>
    where R: io::Read,
          T: DeserializeOwned,
{
    let mut de = Deserializer::new(reader, enum_variant_id);
    let value: T = Deserialize::deserialize(&mut de)?;

    Ok((value, de.reader))
}
