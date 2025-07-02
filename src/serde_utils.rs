use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize, de::Error};


pub(crate) trait APIHistory {
    fn is_missing(&self) -> bool;
}

pub(crate) mod none_as_empty_string {
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, T: Deserialize<'de>, D>(d: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ValueOrEmptyString<'a, T> {
            String(String),
            S(&'a str),
            R(T),
        }

        match ValueOrEmptyString::deserialize(d) {
            Ok(ValueOrEmptyString::R(r)) => Ok(Some(r)),
            Ok(ValueOrEmptyString::S(s)) if s.is_empty() => Ok(None),
            Ok(ValueOrEmptyString::S(_)) => Err(D::Error::custom("only empty strings may be provided")),
            Ok(ValueOrEmptyString::String(s)) if s.is_empty() => Ok(None),
            Ok(ValueOrEmptyString::String(_)) => Err(D::Error::custom("only empty strings may be provided")),
            Err(err) => Err(err),
        }
    }

    pub fn serialize<S, T: Serialize>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match value {
            Some(t) => t.serialize(serializer),
            None => "".serialize(serializer)
        }
    }
}

pub(crate) struct FromStrDeserializer<T: FromStr>(pub(crate) T) where T::Err: Display;
impl<'de, T: FromStr> Deserialize<'de> for FromStrDeserializer<T> where T::Err: Display{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper<'a> {
            String(String),
            Str(&'a str),
        }

        match Helper::deserialize(deserializer) {
            Ok(Helper::String(s)) => T::from_str(&s).map(Self).map_err(|e| D::Error::custom(e)),
            Ok(Helper::Str(s)) => T::from_str(s).map(Self).map_err(|e| D::Error::custom(e)),
            Err(e) => Err(e)
        }
        
    }
}

#[derive(Serialize)]
pub(crate) struct DisplaySerializer(String);
impl<T: Display> From<T> for DisplaySerializer {
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}
