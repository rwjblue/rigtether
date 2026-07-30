//! Strict v0 JSON decoding.

use core::fmt;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

/// Strict logical-message parsing failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonError(String);

impl JsonError {
    /// Stable diagnostic text.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("strict JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value < 0 {
            Err(E::custom("negative integer"))
        } else {
            let value = u64::try_from(value).map_err(E::custom)?;
            Ok(StrictValue(Value::Number(Number::from(value))))
        }
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(E::custom("floating-point number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictValue>()? {
            values.push(value.0);
        }
        Ok(StrictValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some((key, value)) = access.next_entry::<String, StrictValue>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key {key}")));
            }
            values.insert(key, value.0);
        }
        Ok(StrictValue(Value::Object(values)))
    }
}

/// Decode one strict UTF-8 JSON object with integer-only numbers.
///
/// # Errors
///
/// Rejects invalid UTF-8, duplicate keys, floats/exponents, negative values, trailing
/// bytes, and non-object top-level values.
pub fn parse_object(raw: &[u8]) -> Result<Map<String, Value>, JsonError> {
    let text = core::str::from_utf8(raw).map_err(|error| JsonError(error.to_string()))?;
    if contains_negative_zero(text) {
        return Err(JsonError("negative zero".to_owned()));
    }
    let mut deserializer = serde_json::Deserializer::from_str(text);
    deserializer.disable_recursion_limit();
    let stack_safe = serde_stacker::Deserializer::new(&mut deserializer);
    let value =
        StrictValue::deserialize(stack_safe).map_err(|error| JsonError(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| JsonError(error.to_string()))?;
    match value.0 {
        Value::Object(object) => Ok(object),
        _ => Err(JsonError("top level is not an object".to_owned())),
    }
}

fn contains_negative_zero(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index + 1 < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
        } else if byte == b'-' && bytes[index + 1] == b'0' {
            let following = bytes.get(index + 2).copied();
            if following.is_none_or(|value| {
                matches!(
                    value,
                    b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t' | b'.' | b'e' | b'E'
                )
            }) {
                return true;
            }
        }
        index += 1;
    }
    false
}
