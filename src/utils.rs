use std::{any::type_name, fmt::Debug, marker::PhantomData, str::FromStr};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_with::{
    de::DeserializeAsWrap, ser::SerializeAsWrap, serde_as, DeserializeAs, PickFirst, Same,
    SerializeAs,
};
use thiserror::Error;

#[cfg(test)]
pub(crate) use test_utils::*;

use crate::enums::PitchType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Error, Default)]
/// Error for fields where some cashews data is missing the field.
///
/// NOTE: mmolb_parsing only aims to support the latest version of each entity on Cashews. This field is only used when:
/// - Entities are deleted from mmolb, so cashews holds onto an old api version (e.g. deleted teams are missing feeds)
/// - mmolb does not retroactively add a field to old entities (e.g. season 0 games don't have a PitcherEntry field)
#[error("this entity is missing this field, usually because the entity is older than the field")]
pub struct AddedLater;

pub type AddedLaterResult<T> = Result<T, AddedLater>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Error, Default)]
/// Error for fields where some cashews data is missing the field.
///
/// NOTE: mmolb_parsing only aims to support the latest version of each entity on Cashews. This field is only used when:
/// - Entities are deleted from mmolb, so cashews holds onto an old api version (e.g. deleted player's might have items with a prefix field instead of a prefixes fields)
/// - mmolb does not retroactively remove a field from old entities
#[error("this entity is missing this field, usually because the entity is newer than the field")]
pub struct RemovedLater;

pub type RemovedLaterResult<T> = Result<T, RemovedLater>;

pub(crate) struct SometimesMissingHelper<T>(PhantomData<T>);

impl<T> SometimesMissingHelper<T> {
    pub fn default_result<E: Default>() -> Result<T, E> {
        Err(E::default())
    }
}

impl<'de, E, T, U> DeserializeAs<'de, Result<T, E>> for SometimesMissingHelper<U>
where
    U: DeserializeAs<'de, T>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Result<T, E>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Ok(
            DeserializeAsWrap::<T, U>::deserialize(deserializer)?.into_inner()
        ))
    }
}

impl<E, T, U> SerializeAs<Result<T, E>> for SometimesMissingHelper<U>
where
    U: SerializeAs<T>,
{
    fn serialize_as<S>(source: &Result<T, E>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match source.as_ref().ok() {
            Some(a) => SerializeAsWrap::<T, U>::new(a).serialize(serializer),
            None => serializer.serialize_none(),
        }
    }
}

/// serde_as converter for an Option<T>. **This only works when T fails to deserialize from an empty string**
/// because this is optimised to assume usually Some(T) is present, the Some branch goes first.
pub(crate) struct NonStringOrEmptyString;

impl<'de, T: Deserialize<'de>> DeserializeAs<'de, Option<T>> for NonStringOrEmptyString {
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        PickFirst::<(Same, EmptyString)>::deserialize_as(deserializer)
    }
}

impl<T: Serialize> SerializeAs<Option<T>> for NonStringOrEmptyString {
    fn serialize_as<S>(source: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match source {
            Some(t) => t.serialize(serializer),
            None => "".serialize(serializer),
        }
    }
}

/// serde_as converter that expects to always see an empty string. Currently only produces an Option::None value, because it is
/// intended for use as the second branch of NonStringOrEmptyString.
struct EmptyString;

impl<'de, T: Deserialize<'de>> DeserializeAs<'de, Option<T>> for EmptyString {
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EmptyStringVisitor;
        impl<'de> Visitor<'de> for EmptyStringVisitor {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "an empty string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "" => Ok(()),
                    _ => Err(E::custom("not an empty string")),
                }
            }
        }

        deserializer
            .deserialize_str(EmptyStringVisitor)
            .map(|_| None)
    }
}

impl<T: Serialize> SerializeAs<Option<T>> for EmptyString {
    fn serialize_as<S>(source: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match source {
            Some(t) => t.serialize(serializer),
            None => "".serialize(serializer),
        }
    }
}

pub(crate) struct ExpectNone<T>(PhantomData<T>);

impl<'de, T: Debug, U> DeserializeAs<'de, Option<T>> for ExpectNone<U>
where
    U: DeserializeAs<'de, Option<T>>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let result = DeserializeAsWrap::<Option<T>, U>::deserialize(deserializer)?.into_inner();

        if let Some(non_none) = &result {
            tracing::warn!("Expected field to be empty, not to be: {non_none:?}")
        }

        Ok(result)
    }
}

impl<T, U> SerializeAs<Option<T>> for ExpectNone<U>
where
    U: SerializeAs<Option<T>>,
{
    fn serialize_as<S>(source: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerializeAsWrap::<Option<T>, U>::new(source).serialize(serializer)
    }
}

pub(crate) fn extra_fields_deserialize<'de, D>(
    deserializer: D,
) -> Result<serde_json::Map<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let result = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;

    if !result.is_empty() {
        tracing::warn!("Deserialization found extra fields: {:?}", result)
    }

    Ok(result)
}

/// Couldn't parse this value, usually because it's a new mmolb feature we haven't handled yet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Error)]
#[serde(transparent)]
#[error("failed to parse value: {}", .0)]
pub struct NotRecognized(pub serde_json::Value);

pub type MaybeRecognizedResult<T> = Result<T, NotRecognized>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MaybeRecognizedHelper<T>(PhantomData<T>);

pub(crate) fn maybe_recognized_from_str<T: FromStr>(value: &str) -> MaybeRecognizedResult<T> {
    T::from_str(value).map_err(|_| {
        tracing::warn!("{value:?} not recognized as {}", type_name::<T>());
        NotRecognized(serde_json::Value::String(value.to_string()))
    })
}

pub(crate) fn maybe_recognized_to_string<T: ToString>(value: &MaybeRecognizedResult<T>) -> String {
    match value {
        Ok(t) => t.to_string(),
        Err(NotRecognized(v)) => v.to_string(),
    }
}

impl<'de, T, U> DeserializeAs<'de, MaybeRecognizedResult<T>> for MaybeRecognizedHelper<U>
where
    U: DeserializeAs<'de, T>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Result<T, NotRecognized>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum Visitor<T, U> {
            #[serde(untagged)]
            Recognized(
                #[serde(bound(deserialize = "U: DeserializeAs<'de, T>"))] DeserializeAsWrap<T, U>,
            ),
            #[serde(untagged)]
            Other(serde_json::Value),
        }
        match Visitor::<T, U>::deserialize(deserializer) {
            Ok(Visitor::Recognized(t)) => Ok(Ok(t.into_inner())),
            Ok(Visitor::Other(s)) => {
                tracing::warn!("{s:?} not recognized as {}", type_name::<T>());
                Ok(Err(NotRecognized(s)))
            }
            Err(e) => Err(e),
        }
    }
}

impl<T, U> SerializeAs<MaybeRecognizedResult<T>> for MaybeRecognizedHelper<U>
where
    U: SerializeAs<T>,
{
    fn serialize_as<S>(source: &Result<T, NotRecognized>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match source {
            Ok(t) => SerializeAsWrap::<T, U>::new(t).serialize(serializer),
            Err(s) => s.serialize(serializer),
        }
    }
}

pub struct StarHelper;

struct StarVisitor;
impl<'de> Visitor<'de> for StarVisitor {
    type Value = u8;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Up to 255 ⭐s")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if !v.chars().all(|c| c == '⭐') {
            return Err(serde::de::Error::custom(
                "Expected every character in a star string to be '⭐'",
            ));
        }
        Ok(v.chars().count() as u8)
    }
}

impl<'de> DeserializeAs<'de, u8> for StarHelper {
    fn deserialize_as<D>(deserializer: D) -> Result<u8, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StarVisitor)
    }
}

impl SerializeAs<u8> for StarHelper {
    fn serialize_as<S>(source: &u8, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (0..*source)
            .map(|_| '⭐')
            .collect::<String>()
            .serialize(serializer)
    }
}

pub(crate) struct TimestampHelper;
const FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.6f+00:00";

impl<'de> DeserializeAs<'de, DateTime<Utc>> for TimestampHelper {
    fn deserialize_as<D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    }
}

impl SerializeAs<DateTime<Utc>> for TimestampHelper {
    fn serialize_as<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }
}

/// For certain values, mmolb will use both 0 and 0.0. This type exists so those values round trip
#[derive(Debug, Clone, Copy)]
pub enum ZeroOrF64 {
    Zero,
    F64(f64),
}

impl PartialEq for ZeroOrF64 {
    fn eq(&self, other: &Self) -> bool {
        Into::<f64>::into(*self) == Into::<f64>::into(*other)
    }
}

impl<'de> Deserialize<'de> for ZeroOrF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Serialize, Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Int(u8),
            F64(f64),
        }

        match Helper::deserialize(deserializer)? {
            Helper::Int(0) => Ok(ZeroOrF64::Zero),
            Helper::Int(i) => {
                tracing::error!("INTEGER");
                Err(D::Error::custom(format!("Expected int to be 0 not {}", i)))
            }
            Helper::F64(f) => Ok(ZeroOrF64::F64(f)),
        }
    }
}

impl Serialize for ZeroOrF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ZeroOrF64::Zero => serializer.serialize_u8(0),
            ZeroOrF64::F64(f64) => serializer.serialize_f64(*f64),
        }
    }
}

impl From<ZeroOrF64> for f64 {
    fn from(val: ZeroOrF64) -> Self {
        match val {
            ZeroOrF64::Zero => 0.0,
            ZeroOrF64::F64(f64) => f64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmptyArrayOr<T> {
    EmptyArray,
    Value(T),
}

impl<'de, T, U> DeserializeAs<'de, EmptyArrayOr<T>> for EmptyArrayOr<U>
where
    U: DeserializeAs<'de, T>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<EmptyArrayOr<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[serde_as]
        #[derive(Serialize, Deserialize)]
        #[serde(bound(
            serialize = "U: SerializeAs<T>",
            deserialize = "U: DeserializeAs<'de, T>"
        ))]
        #[serde(untagged)]
        enum EmptyArrayOrHelper<T, U> {
            EmptyArray([U; 0]), // PhantomData (de)serializes as null, so leaving it here.
            Value(#[serde_as(as = "U")] T),
        }

        match EmptyArrayOrHelper::<T, U>::deserialize(deserializer)? {
            EmptyArrayOrHelper::EmptyArray(_) => Ok(EmptyArrayOr::EmptyArray),
            EmptyArrayOrHelper::Value(v) => Ok(EmptyArrayOr::Value(v)),
        }
    }
}

impl<T, U> SerializeAs<EmptyArrayOr<T>> for EmptyArrayOr<U>
where
    U: SerializeAs<T>,
{
    fn serialize_as<S>(source: &EmptyArrayOr<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match source {
            EmptyArrayOr::EmptyArray => Vec::<()>::new().serialize(serializer),
            EmptyArrayOr::Value(v) => SerializeAsWrap::<T, U>::new(v).serialize(serializer),
        }
    }
}

pub struct PitchTypeAcronymHelper;

impl<'de> DeserializeAs<'de, PitchType> for PitchTypeAcronymHelper {
    fn deserialize_as<D>(deserializer: D) -> Result<PitchType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let acronym = <&str>::deserialize(deserializer)?;

        PitchType::from_acronym(acronym).map_err(D::Error::custom)
    }
}

impl SerializeAs<PitchType> for PitchTypeAcronymHelper {
    fn serialize_as<S>(source: &PitchType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        source.acronym().serialize(serializer)
    }
}

#[cfg(test)]
mod test_utils {
    use serde::{de::DeserializeOwned, Serialize};
    use std::{fs::File, io::Read, path::Path};
    use tracing::{subscriber::DefaultGuard, Level, Subscriber};
    use tracing_subscriber::{layer::SubscriberExt, Layer};

    pub(crate) fn no_tracing_errs() -> DefaultGuard {
        let subscriber = tracing_subscriber::fmt().finish().with(NoErrorsLayer);
        tracing::subscriber::set_default(subscriber)
    }
    pub(crate) struct NoErrorsLayer;

    impl<S: Subscriber> Layer<S> for NoErrorsLayer {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            assert!(
                *event.metadata().level() < Level::ERROR,
                "Tracing error: {:?}",
                event
            )
        }
    }

    pub(crate) fn assert_round_trip<T: Serialize + DeserializeOwned>(
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
