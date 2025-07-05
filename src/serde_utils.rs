use std::{fmt::Display, str::FromStr};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub(crate) struct AddedLaterMarker(pub bool);

impl AddedLaterMarker {
    pub(crate) fn new<T>(value: &Option<T>) -> Self {
        Self(value.is_none())
    }
    pub(crate) fn wrap<T>(self, value: T) -> Option<T> {
        (!self.0).then_some(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum SomeOrEmptyString<T> {
    Some(T),
    #[default]
    EmptyString
}

impl<T> From<Option<T>> for SomeOrEmptyString<T> {
    fn from(value: Option<T>) -> Self {
        value.map(Self::Some)
            .unwrap_or(Self::EmptyString)
    }
}

impl<T> From<SomeOrEmptyString<T>> for Option<T> {
    fn from(value: SomeOrEmptyString<T>) -> Self {
        match value {
            SomeOrEmptyString::Some(t) => Some(t),
            SomeOrEmptyString::EmptyString => None
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for SomeOrEmptyString<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ValueOrEmptyString<'a, T> {
            String(String),
            S(&'a str),
            R(T),
        }

        match ValueOrEmptyString::deserialize(deserializer) {
            Ok(ValueOrEmptyString::R(r)) => Ok(Self::Some(r)),
            Ok(ValueOrEmptyString::S(s)) if s.is_empty() => Ok(Self::EmptyString),
            Ok(ValueOrEmptyString::S(_)) => Err(D::Error::custom("only empty strings may be provided")),
            Ok(ValueOrEmptyString::String(s)) if s.is_empty() => Ok(Self::EmptyString),
            Ok(ValueOrEmptyString::String(_)) => Err(D::Error::custom("only empty strings may be provided")),
            Err(err) => Err(err),
        }
    }
}

impl<T: Serialize> Serialize for SomeOrEmptyString<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        match self {
            Self::Some(t) => t.serialize(serializer),
            Self::EmptyString => "".serialize(serializer)
        }
    }
}

pub(crate) struct FromStrDeserializer<T: FromStr>(pub(crate) T) where T::Err: Display;
impl<'de, T: FromStr> Deserialize<'de> for FromStrDeserializer<T> where T::Err: Display{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        #[derive(Deserialize)]
        #[serde(untagged, expecting = "Expected a String or &str")]
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
#[serde(transparent)]
pub(crate) struct DisplaySerializer(String);
impl<T: Display> From<T> for DisplaySerializer {
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

#[cfg(test)]
mod test_utils {
    use std::{fs::File, io::Read, path::Path};
    use serde::{de::DeserializeOwned, Serialize};
   
    pub(crate) fn assert_round_trip<T: Serialize + DeserializeOwned>(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = String::new(); 
        File::open(path)?.read_to_string(&mut buf)?;

        let json: serde_json::Value = serde_json::from_str(&buf)?;
        let event: T = serde_json::from_value(json.clone())?;
        let round_trip = serde_json::to_value(&event)?;

        let diff = serde_json_diff::values(json, round_trip);
        assert!(diff.is_none(), "{diff:?}");
        Ok(())
    }
}

#[cfg(test)]
pub(crate) use test_utils::*;