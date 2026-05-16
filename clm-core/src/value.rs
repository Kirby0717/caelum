use std::collections::HashMap;

use serde::{Deserializer, de, ser};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
}

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(v) => serializer.serialize_bool(*v),
            Value::Int(v) => serializer.serialize_i64(*v),
            Value::Uint(v) => serializer.serialize_u64(*v),
            Value::Float(v) => serializer.serialize_f64(*v),
            Value::Str(v) => serializer.serialize_str(v),
            Value::List(v) => v.serialize(serializer),
            Value::Map(v) => v.serialize(serializer),
        }
    }
}

#[derive(Debug)]
pub struct ValueConvertError(String);
impl From<ValueConvertError> for String {
    fn from(value: ValueConvertError) -> Self {
        value.0
    }
}
impl From<std::num::TryFromIntError> for ValueConvertError {
    fn from(value: std::num::TryFromIntError) -> Self {
        Self(value.to_string())
    }
}
impl std::fmt::Display for ValueConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ValueConvertError {}
impl ser::Error for ValueConvertError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self(msg.to_string())
    }
}
impl de::Error for ValueConvertError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self(msg.to_string())
    }
}

pub fn to_value<T: serde::Serialize>(value: &T) -> Result<Value, ValueConvertError> {
    value.serialize(ValueSerializer)
}
pub fn from_value<T: de::DeserializeOwned>(value: Value) -> Result<T, ValueConvertError> {
    T::deserialize(ValueDeserializer(value))
}

pub struct ValueSerializer;
impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = ValueConvertError;
    type SerializeSeq = SerializeValueSeq;
    type SerializeTuple = SerializeValueSeq;
    type SerializeTupleStruct = SerializeValueSeq;
    type SerializeTupleVariant = SerializeValueVariant<SerializeValueSeq>;
    type SerializeMap = SerializeValueMap;
    type SerializeStruct = SerializeValueMap;
    type SerializeStructVariant = SerializeValueVariant<SerializeValueMap>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(v.into())
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(ValueSerializer)
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Str(variant.to_string()))
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(ValueSerializer)
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        Ok(Value::Map(HashMap::from([(
            variant.to_string(),
            value.serialize(ValueSerializer)?,
        )])))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeValueSeq(Vec::with_capacity(len.unwrap_or(0))))
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeValueVariant {
            variant: variant.to_string(),
            inner: self.serialize_seq(Some(len))?,
        })
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeValueMap(
            HashMap::with_capacity(len.unwrap_or(0)),
            None,
        ))
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeValueVariant {
            variant: variant.to_string(),
            inner: self.serialize_map(Some(len))?,
        })
    }
}

pub struct SerializeValueSeq(Vec<Value>);
impl ser::SerializeSeq for SerializeValueSeq {
    type Ok = Value;
    type Error = ValueConvertError;
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.0.push(value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(self.0))
    }
}
impl ser::SerializeTuple for SerializeValueSeq {
    type Ok = Value;
    type Error = ValueConvertError;
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}
impl ser::SerializeTupleStruct for SerializeValueSeq {
    type Ok = Value;
    type Error = ValueConvertError;
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub struct SerializeValueMap(HashMap<String, Value>, Option<String>);
impl ser::SerializeMap for SerializeValueMap {
    type Ok = Value;
    type Error = ValueConvertError;
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let key_value = key.serialize(ValueSerializer)?;
        match key_value {
            Value::Str(s) => {
                self.1 = Some(s);
                Ok(())
            }
            _ => Err(ValueConvertError("map key must be a string".to_string())),
        }
    }
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let key = self.1.take().ok_or_else(|| {
            ValueConvertError("serialize_value called before serialize_key".to_string())
        })?;
        self.0.insert(key, value.serialize(ValueSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.0))
    }
}
impl ser::SerializeStruct for SerializeValueMap {
    type Ok = Value;
    type Error = ValueConvertError;
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeMap::end(self)
    }
}

pub struct SerializeValueVariant<T> {
    variant: String,
    inner: T,
}
impl ser::SerializeTupleVariant for SerializeValueVariant<SerializeValueSeq> {
    type Ok = Value;
    type Error = ValueConvertError;
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(&mut self.inner, value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(HashMap::from([(
            self.variant,
            ser::SerializeSeq::end(self.inner)?,
        )])))
    }
}
impl ser::SerializeStructVariant for SerializeValueVariant<SerializeValueMap> {
    type Ok = Value;
    type Error = ValueConvertError;
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(&mut self.inner, key, value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(HashMap::from([(
            self.variant,
            ser::SerializeMap::end(self.inner)?,
        )])))
    }
}

pub struct ValueDeserializer(Value);
impl<'de> de::Deserializer<'de> for ValueDeserializer {
    type Error = ValueConvertError;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::Int(v) => visitor.visit_i64(v),
            Value::Uint(v) => visitor.visit_u64(v),
            Value::Float(v) => visitor.visit_f64(v),
            Value::Str(v) => visitor.visit_string(v),
            Value::List(v) => visitor.visit_seq(ValueSeqAccess::new(v)),
            Value::Map(v) => visitor.visit_map(ValueMapAccess::new(v)),
        }
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Value::Null => visitor.visit_none(),
            other => visitor.visit_some(ValueDeserializer(other)),
        }
    }
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Value::Str(s) => visitor.visit_enum(ValueEnumAccess::Unit(s)),
            Value::Map(map) => visitor.visit_enum(ValueEnumAccess::WithData(map)),
            _ => Err(ValueConvertError(
                "expected string or map for enum".to_string(),
            )),
        }
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 f32 f64
        char str string bytes byte_buf
        unit unit_struct
        seq tuple tuple_struct
        map struct identifier ignored_any
    }
}

pub struct ValueSeqAccess {
    iter: std::vec::IntoIter<Value>,
}
impl ValueSeqAccess {
    fn new(v: Vec<Value>) -> Self {
        Self {
            iter: v.into_iter(),
        }
    }
}
impl<'de> de::SeqAccess<'de> for ValueSeqAccess {
    type Error = ValueConvertError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, ValueConvertError>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(ValueDeserializer(value)).map(Some),
            None => Ok(None),
        }
    }
}

pub struct ValueMapAccess {
    iter: std::collections::hash_map::IntoIter<String, Value>,
    current_value: Option<Value>,
}
impl ValueMapAccess {
    fn new(v: HashMap<String, Value>) -> Self {
        Self {
            iter: v.into_iter(),
            current_value: None,
        }
    }
}
impl<'de> de::MapAccess<'de> for ValueMapAccess {
    type Error = ValueConvertError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, ValueConvertError>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.current_value = Some(value);
                seed.deserialize(ValueDeserializer(Value::Str(key)))
                    .map(Some)
            }
            None => Ok(None),
        }
    }
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, ValueConvertError>
    where
        V: de::DeserializeSeed<'de>,
    {
        let value = self.current_value.take().unwrap();
        seed.deserialize(ValueDeserializer(value))
    }
}

pub enum ValueEnumAccess {
    Unit(String),
    WithData(HashMap<String, Value>),
}
impl<'de> de::EnumAccess<'de> for ValueEnumAccess {
    type Variant = ValueVariantAccess;
    type Error = ValueConvertError;
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self {
            ValueEnumAccess::Unit(variant_name) => {
                let variant = seed.deserialize(ValueDeserializer(Value::Str(variant_name)))?;
                Ok((variant, ValueVariantAccess::Unit))
            }
            ValueEnumAccess::WithData(mut map) => {
                let Some((key, value)) = map.drain().next() else {
                    return Err(ValueConvertError(
                        "expected single-entry map for enum".to_string(),
                    ));
                };
                let variant = seed.deserialize(ValueDeserializer(Value::Str(key)))?;
                Ok((variant, ValueVariantAccess::WithData(value)))
            }
        }
    }
}

pub enum ValueVariantAccess {
    Unit,
    WithData(Value),
}
impl<'de> de::VariantAccess<'de> for ValueVariantAccess {
    type Error = ValueConvertError;
    fn unit_variant(self) -> Result<(), Self::Error> {
        match self {
            ValueVariantAccess::Unit => Ok(()),
            _ => Err(ValueConvertError("expected unit variant".to_string())),
        }
    }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self {
            ValueVariantAccess::WithData(value) => seed.deserialize(ValueDeserializer(value)),
            _ => Err(ValueConvertError("expected newtype variant".to_string())),
        }
    }
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            ValueVariantAccess::WithData(value) => {
                ValueDeserializer(value).deserialize_seq(visitor)
            }
            _ => Err(ValueConvertError("expected tuple variant".to_string())),
        }
    }
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            ValueVariantAccess::WithData(value) => {
                ValueDeserializer(value).deserialize_map(visitor)
            }
            _ => Err(ValueConvertError("expected tuple variant".to_string())),
        }
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}
impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}
impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Value::Int(value as i64)
    }
}
impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value::Uint(value as u64)
    }
}
impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Value::Uint(value as u64)
    }
}
impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Uint(value as u64)
    }
}
impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::Uint(value)
    }
}
impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Value::Uint(value as u64)
    }
}
impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Float(value as f64)
    }
}
impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value)
    }
}
impl From<char> for Value {
    fn from(value: char) -> Self {
        Value::Str(value.to_string())
    }
}
impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Str(value.to_string())
    }
}
impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Value::Str(value.clone())
    }
}
impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Str(value)
    }
}
impl From<&std::path::Path> for Value {
    fn from(value: &std::path::Path) -> Self {
        Value::Str(value.to_string_lossy().to_string())
    }
}
impl From<&std::path::PathBuf> for Value {
    fn from(value: &std::path::PathBuf) -> Self {
        Value::Str(value.to_string_lossy().to_string())
    }
}
impl From<std::path::PathBuf> for Value {
    fn from(value: std::path::PathBuf) -> Self {
        Value::Str(value.to_string_lossy().to_string())
    }
}
impl<T> From<Option<T>> for Value
where
    Value: From<T>,
{
    fn from(value: Option<T>) -> Self {
        if let Some(value) = value {
            value.into()
        } else {
            Value::Null
        }
    }
}
impl<T: Clone, const N: usize> From<&[T; N]> for Value
where
    Value: From<T>,
{
    fn from(value: &[T; N]) -> Self {
        Value::List(value.iter().cloned().map(Value::from).collect())
    }
}
impl<T, const N: usize> From<[T; N]> for Value
where
    Value: From<T>,
{
    fn from(value: [T; N]) -> Self {
        Value::List(value.into_iter().map(Value::from).collect())
    }
}
impl<T: Clone> From<&[T]> for Value
where
    Value: From<T>,
{
    fn from(value: &[T]) -> Self {
        Value::List(value.iter().cloned().map(Value::from).collect())
    }
}
impl<T: Clone> From<&Vec<T>> for Value
where
    Value: From<T>,
{
    fn from(value: &Vec<T>) -> Self {
        Value::List(value.iter().cloned().map(Value::from).collect())
    }
}
impl<T> From<Vec<T>> for Value
where
    Value: From<T>,
{
    fn from(value: Vec<T>) -> Self {
        Value::List(value.into_iter().map(Value::from).collect())
    }
}
impl<T> From<HashMap<String, T>> for Value
where
    Value: From<T>,
{
    fn from(value: HashMap<String, T>) -> Self {
        Value::Map(
            value
                .into_iter()
                .map(|(k, v)| (k, Value::from(v)))
                .collect(),
        )
    }
}

impl TryFrom<Value> for () {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if Value::Null == value {
            Ok(())
        } else {
            Err(ValueConvertError("expected null".to_string()))
        }
    }
}
impl TryFrom<Value> for bool {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Bool(value) = value {
            Ok(value)
        } else {
            Err(ValueConvertError("expected bool".to_string()))
        }
    }
}
impl TryFrom<Value> for i8 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected int".to_string()))
        }
    }
}
impl TryFrom<Value> for i16 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected int".to_string()))
        }
    }
}
impl TryFrom<Value> for i32 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected int".to_string()))
        }
    }
}
impl TryFrom<Value> for i64 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value)
        } else {
            Err(ValueConvertError("expected int".to_string()))
        }
    }
}
impl TryFrom<Value> for isize {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected int".to_string()))
        }
    }
}
impl TryFrom<Value> for u8 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Uint(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected uint".to_string()))
        }
    }
}
impl TryFrom<Value> for u16 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Uint(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected uint".to_string()))
        }
    }
}
impl TryFrom<Value> for u32 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Uint(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected uint".to_string()))
        }
    }
}
impl TryFrom<Value> for u64 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Uint(value) = value {
            Ok(value)
        } else {
            Err(ValueConvertError("expected uint".to_string()))
        }
    }
}
impl TryFrom<Value> for usize {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Uint(value) = value {
            Ok(Self::try_from(value)?)
        } else {
            Err(ValueConvertError("expected uint".to_string()))
        }
    }
}
impl TryFrom<Value> for f32 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Float(value) = value {
            Ok(value as Self)
        } else {
            Err(ValueConvertError("expected float".to_string()))
        }
    }
}
impl TryFrom<Value> for f64 {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Float(value) = value {
            Ok(value)
        } else {
            Err(ValueConvertError("expected float".to_string()))
        }
    }
}
impl TryFrom<Value> for char {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Str(value) = value {
            let chars = value.chars().take(2).collect::<Vec<_>>();
            if chars.len() == 1 {
                Ok(chars[0])
            } else {
                Err(ValueConvertError("string length is not one".to_string()))
            }
        } else {
            Err(ValueConvertError("expected string".to_string()))
        }
    }
}
impl TryFrom<Value> for String {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Str(value) = value {
            Ok(value)
        } else {
            Err(ValueConvertError("expected string".to_string()))
        }
    }
}
impl<T: TryFrom<Value, Error = ValueConvertError>> TryFrom<Value> for Option<T> {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value == Value::Null {
            Ok(None)
        } else {
            Ok(Some(value.try_into()?))
        }
    }
}
impl<T: TryFrom<Value, Error = ValueConvertError>> TryFrom<Value> for Vec<T> {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::List(value) = value {
            Ok(value
                .into_iter()
                .map(T::try_from)
                .collect::<Result<Vec<T>, ValueConvertError>>()?)
        } else {
            Err(ValueConvertError("expected list".to_string()))
        }
    }
}
impl<T: TryFrom<Value, Error = ValueConvertError>, const N: usize> TryFrom<Value> for [T; N] {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::List(value) = value {
            Ok(value
                .into_iter()
                .map(T::try_from)
                .collect::<Result<Vec<T>, ValueConvertError>>()?
                .try_into()
                .map_err(|_| ValueConvertError("arity mismatch".to_string()))?)
        } else {
            Err(ValueConvertError("expected list".to_string()))
        }
    }
}
impl<T: TryFrom<Value, Error = ValueConvertError>> TryFrom<Value> for HashMap<String, T> {
    type Error = ValueConvertError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Map(value) = value {
            Ok(value
                .into_iter()
                .map(|(key, value)| Ok((key, T::try_from(value)?)))
                .collect::<Result<HashMap<String, T>, ValueConvertError>>()?)
        } else {
            Err(ValueConvertError("expected map".to_string()))
        }
    }
}

macro_rules! impl_tuple_value {
    ($($name:ident),+) => {
        impl<$($name),+> From<($($name,)+)> for Value
        where $(Value: From<$name>),+
        {
            fn from(value: ($($name,)+)) -> Self {
                #[allow(non_snake_case)]
                let ($($name,)+) = value;
                Value::List(vec![$(Value::from($name)),+])
            }
        }
        impl<$($name),+> TryFrom<Value> for ($($name,)+)
        where $($name: TryFrom<Value, Error = ValueConvertError>),+
        {
            type Error = ValueConvertError;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                let Value::List(value) = value else {
                    return Err(ValueConvertError("expected list".to_string()));
                };
                #[allow(non_snake_case)]
                let Ok([$($name),+]): Result<[_; _], _> = value.try_into() else {
                    return Err(ValueConvertError("arity mismatch".to_string()));
                };
                Ok(($($name.try_into()?,)+))
            }
        }
    };
}
macro_rules! impl_tuples {
    ($($acc:ident),* ;) => {};
    ($($acc:ident),* ; $head:ident $(, $tail:ident)*) => {
        impl_tuple_value!($($acc,)* $head);
        impl_tuples!($($acc,)* $head ; $($tail),*);
    };
}
impl_tuples!(;T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
