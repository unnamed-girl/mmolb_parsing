use std::{any::type_name, fmt::{Debug, Display}, marker::PhantomData, str::FromStr};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{de::DeserializeAsWrap, ser::SerializeAsWrap, DeserializeAs, SerializeAs};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AddedLater;

impl Display for AddedLater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Old data that doesn't contain this field")
    }
}

impl std::error::Error for AddedLater {}

pub type AddedLaterResult<T> = Result<T, AddedLater>;


pub struct AddedLaterHelper<T>(PhantomData<T>);

impl<T> AddedLaterHelper<T> {
    pub fn default_result() -> AddedLaterResult<T> {
        AddedLaterResult::Err(AddedLater)
    }
}

impl<'de, T, U> DeserializeAs<'de, AddedLaterResult<T>> for AddedLaterHelper<U>
where U: DeserializeAs<'de, T> {
    fn deserialize_as<D>(deserializer: D) -> Result<AddedLaterResult<T>, D::Error>
        where
            D: Deserializer<'de> {
        Ok(Ok(DeserializeAsWrap::<T, U>::deserialize(deserializer)?.into_inner()))
    }
}

impl<T, U> SerializeAs<AddedLaterResult<T>> for AddedLaterHelper<U> 
where U: SerializeAs<T> {
    fn serialize_as<S>(source: &AddedLaterResult<T>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        match source.as_ref().ok() {
            Some(a) => SerializeAsWrap::<T, U>::new(a).serialize(serializer),
            None => serializer.serialize_none()
        }
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

pub(crate) struct ExpectNone<T>(PhantomData<T>);

impl<'de, T: Debug, U> DeserializeAs<'de, Option<T>> for ExpectNone<U> 
    where U: DeserializeAs<'de, Option<T>> {
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            D: Deserializer<'de> {
        let result = DeserializeAsWrap::<Option::<T>, U>::deserialize(deserializer)?.into_inner();

        if let Some(non_none) = &result {
            tracing::error!("Expected field to be empty, not to be: {non_none:?}")
        }
    
        Ok(result)
    }
}

impl<T, U> SerializeAs<Option<T>> for ExpectNone<U>
    where U: SerializeAs<Option<T>> {
    fn serialize_as<S>(source: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        SerializeAsWrap::<Option<T>, U>::new(source).serialize(serializer)
    }
}

pub(crate) fn extra_fields_deserialize<'de, D>(deserializer: D) -> Result<serde_json::Map<String, serde_json::Value>, D::Error>
    where
        D: Deserializer<'de> {
    let result = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;

    if !result.is_empty() {
        tracing::error!("Deserialization found extra fields: {:?}", result)
    }

    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
/// When trying to parse the input, found a value we haven't added to our data model yet.
pub struct NotRecognized(pub serde_json::Value);

impl Display for NotRecognized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let received = self.0.to_string();
        write!(f, "Failed to parse {received}")
    }
}

impl std::error::Error for NotRecognized {}

pub type MaybeRecognizedResult<T> = Result<T, NotRecognized>;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaybeRecognizedHelper<T>(PhantomData<T>);

pub(crate) fn maybe_recognized_from_str<T: FromStr>(value: &str) -> MaybeRecognizedResult<T> {
    T::from_str(value).map_err(|_| NotRecognized(serde_json::Value::String(value.to_string())))
}

pub(crate) fn maybe_recognized_to_string<T: ToString>(value: &MaybeRecognizedResult<T>) -> String {
    match value {
        Ok(t) => t.to_string(),
        Err(NotRecognized(v)) => v.to_string()
    }
}


impl<'de, T, U> DeserializeAs<'de, MaybeRecognizedResult<T>> for MaybeRecognizedHelper<U> 
    where U: DeserializeAs<'de, T>{
    fn deserialize_as<D>(deserializer: D) -> Result<Result<T, NotRecognized>, D::Error>
        where
            D: Deserializer<'de> {

        #[derive(Deserialize)]
        enum Visitor<T, U> {
            #[serde(untagged)]
            Recognized(#[serde(bound(deserialize = "U: DeserializeAs<'de, T>"))] DeserializeAsWrap<T, U>),
            #[serde(untagged)]
            Other(serde_json::Value)
        }
        match Visitor::<T, U>::deserialize(deserializer) {
            Ok(Visitor::Recognized(t)) => Ok(Ok(t.into_inner())),
            Ok(Visitor::Other(s)) => {
                tracing::error!("{s:?} not recognized as {}", type_name::<T>());
                Ok(Err(NotRecognized(s, )))
            }
            Err(e) => Err(e)
        }
    }
}

impl<T, U> SerializeAs<MaybeRecognizedResult<T>> for MaybeRecognizedHelper<U> 
    where U: SerializeAs<T> {
    fn serialize_as<S>(source: &Result<T, NotRecognized>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        match source {
            Ok(t) => SerializeAsWrap::<T, U>::new(t).serialize(serializer),
            Err(s) => s.serialize(serializer)
        }
    }
}

#[cfg(test)]
mod test_utils {
    use std::{fs::File, io::Read, path::Path};
    use serde::{de::DeserializeOwned, Serialize};
    use tracing::{subscriber::DefaultGuard, Level, Subscriber};
    use tracing_subscriber::{layer::SubscriberExt, Layer};

    pub(crate) fn no_tracing_errs() -> DefaultGuard {
        let subscriber = tracing_subscriber::fmt().finish().with(NoErrorsLayer);
        tracing::subscriber::set_default(subscriber)
    }
    pub(crate) struct NoErrorsLayer;

    impl<S: Subscriber> Layer<S> for NoErrorsLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
            assert!(*event.metadata().level() < Level::ERROR, "Tracing error: {:?}", event)
        }
    }
   
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
