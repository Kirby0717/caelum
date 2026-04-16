use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Error(String),
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}
impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Value::Int(value as i64)
    }
}
impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Value::Int(value as i64)
    }
}
impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value as i64)
    }
}
impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::Int(value as i64)
    }
}
impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}
impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Value::Int(value as i64)
    }
}
impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Value::Int(value as i64)
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
impl<T> From<Option<T>> for Value
where
    Value: From<T>,
{
    fn from(value: Option<T>) -> Self {
        if let Some(value) = value {
            value.into()
        }
        else {
            Value::Null
        }
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

impl TryFrom<Value> for bool {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Bool(value) = value {
            Ok(value)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for u8 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for i8 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for u16 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for i16 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for u32 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for i32 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for u64 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for i64 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for usize {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for isize {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Int(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for f32 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Float(value) = value {
            Ok(value as Self)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for f64 {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Float(value) = value {
            Ok(value)
        }
        else {
            Err(())
        }
    }
}
impl TryFrom<Value> for String {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Str(value) = value {
            Ok(value)
        }
        else {
            Err(())
        }
    }
}
impl<T: TryFrom<Value, Error = ()>> TryFrom<Value> for Option<T> {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value == Value::Null {
            Ok(None)
        }
        else {
            Ok(Some(value.try_into()?))
        }
    }
}
impl<T: TryFrom<Value, Error = ()>> TryFrom<Value> for Vec<T> {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::List(value) = value {
            Ok(value
                .into_iter()
                .map(T::try_from)
                .collect::<Result<Vec<T>, ()>>()?)
        }
        else {
            Err(())
        }
    }
}
impl<T: TryFrom<Value, Error = ()>> TryFrom<Value> for HashMap<String, T> {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Map(value) = value {
            Ok(value
                .into_iter()
                .map(|(key, value)| Ok((key, T::try_from(value)?)))
                .collect::<Result<HashMap<String, T>, ()>>()?)
        }
        else {
            Err(())
        }
    }
}
