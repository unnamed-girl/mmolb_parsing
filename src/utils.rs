use std::ops::{Deref, DerefMut};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddedLater<T>(T, bool);
impl<T> AddedLater<T> {
    pub(crate) fn skip(&self) -> bool {
        !self.1
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl<T: Serialize> Serialize for AddedLater<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        self.0.serialize(serializer)
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for AddedLater<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de> {
        Ok(Self(T::deserialize(deserializer)?, true))
    }
}

impl<T: Default> Default for AddedLater<T> {
    fn default() -> Self {
        Self(T::default(), false)
    }
}

impl<T> Deref for AddedLater<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for AddedLater<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemovedLater<T>(T, bool);
impl<T> RemovedLater<T> {
    pub(crate) fn skip(&self) -> bool {
        !self.1
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl<T: Serialize> Serialize for RemovedLater<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        self.0.serialize(serializer)
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for RemovedLater<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de> {
        Ok(Self(T::deserialize(deserializer)?, true))
    }
}

impl<T: Default> Default for RemovedLater<T> {
    fn default() -> Self {
        Self(T::default(), false)
    }
}

impl<T> Deref for RemovedLater<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for RemovedLater<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct ExpectNone(Option<serde_json::Value>);

impl<'de> Deserialize<'de> for ExpectNone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de> {
        let result = Option::<serde_json::Value>::deserialize(deserializer)?;

        if let Some(non_none) = &result {
            tracing::error!("Expected field to be empty, not to be: {non_none:?}")
        }
        
        Ok(Self(result))
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