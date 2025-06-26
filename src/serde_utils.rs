
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
