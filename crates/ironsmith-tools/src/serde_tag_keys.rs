//! Enumerates the `TagKey`s inside any `Serialize` value.
//!
//! `TagKey` serializes as the newtype struct `TagKey`; this serializer walks a
//! value's serde representation and collects the payload of every such
//! newtype, so the runtime definition's keys can be listed without the
//! runtime types implementing a traversal of their own.

use serde::ser::{
    self, Impossible, Serialize, SerializeMap, SerializeSeq, SerializeStruct,
    SerializeStructVariant, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use std::fmt;

/// The keys `value` carries, in serialization order, duplicates kept.
pub fn tag_keys_of_serializable<T: Serialize + ?Sized>(value: &T) -> Vec<String> {
    let mut keys = Vec::new();
    let _ = value.serialize(Collector { keys: &mut keys, in_key: false });
    keys
}

#[derive(Debug)]
struct Never;

impl fmt::Display for Never {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("tag key collection cannot fail")
    }
}

impl std::error::Error for Never {}

impl ser::Error for Never {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Never
    }
}

struct Collector<'a> {
    keys: &'a mut Vec<String>,
    /// Inside the `TagKey` newtype: the next string is a key.
    in_key: bool,
}

impl<'a> Collector<'a> {
    fn child(&mut self) -> Collector<'_> {
        Collector { keys: self.keys, in_key: false }
    }
}

macro_rules! leaf {
    ($($method:ident: $ty:ty),* $(,)?) => {
        $(fn $method(self, _v: $ty) -> Result<(), Never> { Ok(()) })*
    };
}

impl<'a> ser::Serializer for Collector<'a> {
    type Ok = ();
    type Error = Never;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    leaf!(
        serialize_bool: bool, serialize_i8: i8, serialize_i16: i16, serialize_i32: i32,
        serialize_i64: i64, serialize_i128: i128, serialize_u8: u8, serialize_u16: u16,
        serialize_u32: u32, serialize_u64: u64, serialize_u128: u128, serialize_f32: f32,
        serialize_f64: f64, serialize_char: char, serialize_bytes: &[u8],
    );

    fn serialize_str(self, v: &str) -> Result<(), Never> {
        if self.in_key {
            self.keys.push(v.to_string());
        }
        Ok(())
    }
    fn serialize_none(self) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(mut self, value: &T) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn serialize_unit(self) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_unit_variant(self, _n: &'static str, _i: u32, _v: &'static str) -> Result<(), Never> {
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Never> {
        let in_key = name == "TagKey";
        value.serialize(Collector { keys: self.keys, in_key })
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        mut self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        value: &T,
    ) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self, Never> {
        Ok(self)
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self, Never> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _n: &'static str, _len: usize) -> Result<Self, Never> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self, Never> {
        Ok(self)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self, Never> {
        Ok(self)
    }
    fn serialize_struct(self, _n: &'static str, _len: usize) -> Result<Self, Never> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self, Never> {
        Ok(self)
    }
    fn collect_str<T: fmt::Display + ?Sized>(self, value: &T) -> Result<(), Never> {
        if self.in_key {
            self.keys.push(value.to_string());
        }
        Ok(())
    }
}

impl<'a> SerializeSeq for Collector<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl<'a> SerializeTuple for Collector<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl<'a> SerializeTupleStruct for Collector<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl<'a> SerializeTupleVariant for Collector<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl<'a> SerializeMap for Collector<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Never> {
        key.serialize(self.child())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl<'a> SerializeStruct for Collector<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

impl<'a> SerializeStructVariant for Collector<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Never> {
        value.serialize(self.child())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

#[allow(dead_code)]
type Unused = Impossible<(), Never>;
