use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fmt;
use error::*;
use serde::de::{Deserialize, Deserializer, Visitor};

use config::Config;

/// Underlying kind of the configuration value.
#[derive(Debug, Clone)]
pub enum ValueKind {
    Nil,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Table(Table),
    Array(Array),
}

pub type Array = Vec<Value>;
pub type Table = HashMap<String, Value>;

impl Default for ValueKind {
    fn default() -> Self {
        ValueKind::Nil
    }
}

impl<T> From<Option<T>> for ValueKind
    where T: Into<ValueKind>
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => ValueKind::Nil,
        }
    }
}

impl From<String> for ValueKind {
    fn from(value: String) -> Self {
        ValueKind::String(value.into())
    }
}

impl<'a> From<&'a str> for ValueKind {
    fn from(value: &'a str) -> Self {
        ValueKind::String(value.into())
    }
}

impl From<i64> for ValueKind {
    fn from(value: i64) -> Self {
        ValueKind::Integer(value)
    }
}

impl From<f64> for ValueKind {
    fn from(value: f64) -> Self {
        ValueKind::Float(value)
    }
}

impl From<bool> for ValueKind {
    fn from(value: bool) -> Self {
        ValueKind::Boolean(value)
    }
}

impl<T> From<HashMap<String, T>> for ValueKind
    where T: Into<Value>
{
    fn from(values: HashMap<String, T>) -> Self {
        let mut r = HashMap::new();

        for (k, v) in values {
            r.insert(k.clone(), v.into());
        }

        ValueKind::Table(r)
    }
}

impl<T> From<Vec<T>> for ValueKind
    where T: Into<Value>
{
    fn from(values: Vec<T>) -> Self {
        let mut l = Vec::new();

        for v in values {
            l.push(v.into());
        }

        ValueKind::Array(l)
    }
}

/// A configuration value.
#[derive(Default, Debug, Clone)]
pub struct Value {
    /// A description of the original location of the value.
    ///
    /// A Value originating from a File might contain:
    /// ```
    /// Settings.toml
    /// ```
    ///
    /// A Value originating from the environment would contain:
    /// ```
    /// the envrionment
    /// ```
    ///
    /// A Value originating from a remote source might contain:
    /// ```
    /// etcd+http://127.0.0.1:2379
    /// ```
    origin: Option<String>,

    /// Underlying kind of the configuration value.
    pub kind: ValueKind,
}

impl Value {
    /// Create a new value instance that will remember its source uri.
    pub fn new<V>(origin: Option<&String>, kind: V) -> Self
        where V: Into<ValueKind>
    {
        Value {
            origin: origin.cloned(),
            kind: kind.into(),
        }
    }

    pub fn try_into<'de, T: Deserialize<'de>>(self) -> Result<T> {
        T::deserialize(self)
    }

    pub fn into_bool(self) -> Result<bool> {
        match self.kind {
            ValueKind::Boolean(value) => Ok(value),
            ValueKind::Integer(value) => Ok(value != 0),
            ValueKind::Float(value) => Ok(value != 0.0),

            ValueKind::String(value) => {
                match value.to_lowercase().as_ref() {
                    "1" | "true" | "on" | "yes" => Ok(true),
                    "0" | "false" | "off" | "no" => Ok(false),

                    // Unexpected string value
                    s => {
                        Err(ConfigError::invalid_type(self.origin.clone(),
                                                      ValueKind::String(value),
                                                      "a boolean"))
                    }
                }
            }

            // Unexpected type
            kind => {
                Err(ConfigError::invalid_type(self.origin.clone(), kind, "a boolean"))
            }
        }
    }

    pub fn into_int(self) -> Result<i64> {
        match self.kind {
            ValueKind::Integer(value) => Ok(value),

            ValueKind::String(ref s) => {
                match s.to_lowercase().as_ref() {
                    "true" | "on" | "yes" => Ok(1),
                    "false" | "off" | "no" => Ok(0),
                    _ => {
                        s.parse().map_err(|_| {
                                              // Unexpected string
                                              ConfigError::invalid_type(self.origin.clone(),
                                                                        ValueKind::String(s.clone()),
                                                                        "an integer")
                                          })
                    }
                }
            }

            ValueKind::Boolean(value) => Ok(if value { 1 } else { 0 }),
            ValueKind::Float(value) => Ok(value.round() as i64),

            // Unexpected type
            kind => Err(ConfigError::invalid_type(self.origin.clone(), kind, "an integer"))
        }
    }

    pub fn into_float(self) -> Result<f64> {
        match self.kind {
            ValueKind::Float(value) => Ok(value),

            ValueKind::String(ref s) => {
                match s.to_lowercase().as_ref() {
                    "true" | "on" | "yes" => Ok(1.0),
                    "false" | "off" | "no" => Ok(0.0),
                    _ => {
                        s.parse().map_err(|_| {
                                              // Unexpected string
                                              ConfigError::invalid_type(self.origin.clone(),
                                                                        ValueKind::String(s.clone()),
                                                                        "a floating point")
                                          })
                    }
                }
            }

            ValueKind::Integer(value) => Ok(value as f64),
            ValueKind::Boolean(value) => Ok(if value { 1.0 } else { 0.0 }),

            // Unexpected type
            kind => Err(ConfigError::invalid_type(self.origin, kind, "a floating point"))
        }
    }

    pub fn into_str(self) -> Result<String> {
        match self.kind {
            ValueKind::String(value) => Ok(value),

            ValueKind::Boolean(value) => Ok(value.to_string()),
            ValueKind::Integer(value) => Ok(value.to_string()),
            ValueKind::Float(value) => Ok(value.to_string()),

            // Cannot convert
            kind => Err(ConfigError::invalid_type(self.origin, kind, "a string"))
        }
    }

    pub fn into_array(self) -> Result<Vec<Value>> {
        match self.kind {
            ValueKind::Array(value) => Ok(value),

            // Cannot convert
            kind => Err(ConfigError::invalid_type(self.origin, kind, "an array"))
        }
    }

    pub fn into_tree(self) -> Result<Config> {
        match self.kind {
            ValueKind::Table(value) => Ok(Config::from(value)),

            // Cannot convert
            kind => Err(ConfigError::invalid_type(self.origin, kind, "a map"))
        }
    }
    
    pub fn as_string(&self) -> String {
        match self.kind {
            ValueKind::Nil => { "".to_string() },
            ValueKind::Boolean(ref b) => format!("{}", match b { &true => "true", &false => "false" }),
            ValueKind::Integer(ref i) => format!("{}", i),
            ValueKind::Float(ref f) => format!("{}", f),
            ValueKind::String(ref s) => format!("{}", s),
            ValueKind::Table(ref t) => {
                let mut sorted_vec = t.iter().map(|(k, v)| {
                    format!("{}: {}", k, v)
                }).collect::<Vec<String>>();
                sorted_vec.sort();
                format!("{{ {} }}", sorted_vec.join(", "))
            },
            ValueKind::Array(ref a) => {
                format!("[ {} ]", a.iter().map(|i| {
                    i.as_string()
                }).collect::<Vec<String>>().join(", "))
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any valid configuration value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> ::std::result::Result<Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_i8<E>(self, value: i8) -> ::std::result::Result<Value, E> {
                Ok((value as i64).into())
            }

            #[inline]
            fn visit_i16<E>(self, value: i16) -> ::std::result::Result<Value, E> {
                Ok((value as i64).into())
            }

            #[inline]
            fn visit_i32<E>(self, value: i32) -> ::std::result::Result<Value, E> {
                Ok((value as i64).into())
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> ::std::result::Result<Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_u8<E>(self, value: u8) -> ::std::result::Result<Value, E> {
                Ok((value as i64).into())
            }

            #[inline]
            fn visit_u16<E>(self, value: u16) -> ::std::result::Result<Value, E> {
                Ok((value as i64).into())
            }

            #[inline]
            fn visit_u32<E>(self, value: u32) -> ::std::result::Result<Value, E> {
                Ok((value as i64).into())
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> ::std::result::Result<Value, E> {
                // FIXME: This is bad
                Ok((value as i64).into())
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> ::std::result::Result<Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> ::std::result::Result<Value, E>
            where
                E: ::serde::de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> ::std::result::Result<Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_none<E>(self) -> ::std::result::Result<Value, E> {
                Ok(Value::new(None, ValueKind::Nil))
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> ::std::result::Result<Value, D::Error>
                where D: Deserializer<'de>
            {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> ::std::result::Result<Value, E> {
                Ok(Value::new(None, ValueKind::Nil))
            }

            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> ::std::result::Result<Value, V::Error>
            where
                V: ::serde::de::SeqAccess<'de>,
            {
                let mut vec = Array::new();

                while let Some(elem) = try!(visitor.next_element()) {
                    vec.push(elem);
                }

                Ok(vec.into())
            }

            fn visit_map<V>(self, mut visitor: V) -> ::std::result::Result<Value, V::Error>
            where
                V: ::serde::de::MapAccess<'de>,
            {
                let mut values = Table::new();

                while let Some((key, value)) = try!(visitor.next_entry()) {
                    values.insert(key, value);
                }

                Ok(values.into())
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<T> From<T> for Value
    where T: Into<ValueKind>
{
    fn from(value: T) -> Self {
        Value {
            origin: None,
            kind: value.into(),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.write_fmt(format_args!("{}", self.as_string()))
    }
}

pub struct ValueWithKey<'a>(pub Value, &'a str);

impl<'a> ValueWithKey<'a> {
    pub fn new(value: Value, key: &'a str) -> Self
    {
        ValueWithKey(value, key)
    }

    pub fn into_bool(self) -> Result<bool> {
        match self.0.into_bool() {
            Ok(value) => Ok(value),
            Err(error) => Err(error.extend_with_key(self.1))
        }
    }

    /// Returns `self` into an i64, if possible.
    pub fn into_int(self) -> Result<i64> {
        match self.0.into_int() {
            Ok(value) => Ok(value),
            Err(error) => Err(error.extend_with_key(self.1))
        }
    }

    /// Returns `self` into a f64, if possible.
    pub fn into_float(self) -> Result<f64> {
        match self.0.into_float() {
            Ok(value) => Ok(value),
            Err(error) => Err(error.extend_with_key(self.1))
        }
    }

    /// Returns `self` into a str, if possible.
    pub fn into_str(self) -> Result<String> {
        match self.0.into_str() {
            Ok(value) => Ok(value),
            Err(error) => Err(error.extend_with_key(self.1))
        }
    }

    /// Returns `self` into an array, if possible
    pub fn into_array(self) -> Result<Vec<Value>> {
        match self.0.into_array() {
            Ok(value) => Ok(value),
            Err(error) => Err(error.extend_with_key(self.1))
        }
    }

    /// If the `Value` is a Table, returns the associated Config.
    pub fn into_tree(self) -> Result<Config> {
        match self.0.into_tree() {
            Ok(value) => Ok(value),
            Err(error) => Err(error.extend_with_key(self.1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_str_as_string() {
        let v_s: Value = Value::new(None, ValueKind::String(format!("test_str")));
        assert_eq!(v_s.as_string(), format!("test_str"));
    }
    
    #[test]
    fn test_int_as_string() {
        let v_i: Value = Value::new(None, ValueKind::Integer(11));
        assert_eq!(v_i.as_string(), format!("11"));
    }
    
    #[test]
    fn test_bool_as_string() {
        let v_b: Value = Value::new(None, ValueKind::Boolean(true));
        assert_eq!(v_b.as_string(), format!("true"));
    }
    
    #[test]
    fn test_table_as_string() {
        let mut inner_table: HashMap<String, Value> = HashMap::new();
        inner_table.insert(format!("key_a"), Value::new(None, ValueKind::String(format!("val1"))));
        inner_table.insert(format!("key_b"), Value::new(None, ValueKind::String(format!("val2"))));
    
        let mut outer_table: HashMap<String, Value> = HashMap::new();
        outer_table.insert(format!("key1"), Value::new(None, ValueKind::String(format!("val1"))));
        outer_table.insert(format!("key2"), Value::new(None, ValueKind::Table(inner_table)));
    
        let v_t: Value = Value::new(None, ValueKind::Table(outer_table));
        assert_eq!(v_t.as_string(), format!("{{ key1: val1, key2: {{ key_a: val1, key_b: val2 }} }}"));
    }
    
    #[test]
    fn test_array_as_string() {
        let mut test_array: Vec<Value> = Vec::new();
    
        test_array.push(Value::new(None, ValueKind::String(format!("test_str1"))));
        test_array.push(Value::new(None, ValueKind::Integer(22)));
        
        let v_a: Value = Value::new(None, ValueKind::Array(test_array));
        assert_eq!(v_a.as_string(), format!("[ test_str1, 22 ]"));
    }
    
    #[test]
    fn test_complex_table_as_string() {
        let mut array_in_table: Vec<Value> = Vec::new();
        array_in_table.push(Value::new(None, ValueKind::String(format!("test"))));
        array_in_table.push(Value::new(None, ValueKind::Integer(22)));
    
        let mut table_with_array: HashMap<String, Value> = HashMap::new();
        table_with_array.insert(format!("key_a"), Value::new(None, ValueKind::String(format!("test2"))));
        table_with_array.insert(format!("key_b"), Value::new(None, ValueKind::Array(array_in_table)));
    
        let mut table_in_array: HashMap<String, Value> = HashMap::new();
        table_in_array.insert(format!("key1"), Value::new(None, ValueKind::String(format!("test2"))));
        table_in_array.insert(format!("key2"), Value::new(None, ValueKind::Integer(33)));

        let mut array_with_table: Vec<Value> = Vec::new();
        array_with_table.push(Value::new(None, ValueKind::String(format!("test3"))));
        array_with_table.push(Value::new(None, ValueKind::Table(table_in_array)));
    
        let mut outer_table_complex: HashMap<String, Value> = HashMap::new();
        outer_table_complex.insert(format!("att"),
                                   Value::new(None, ValueKind::Table(table_with_array)));
        outer_table_complex.insert(format!("tat"),
                                   Value::new(None, ValueKind::Array(array_with_table)));
    
        let v_tc: Value = Value::new(None, ValueKind::Table(outer_table_complex));
        assert_eq!(v_tc.as_string(),
                   format!("{{ att: {{ key_a: test2, key_b: [ test, 22 ] }}, \
                              tat: [ test3, {{ key1: test2, key2: 33 }} ] }}"));
    
    }
}