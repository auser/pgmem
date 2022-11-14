use serde::{Deserialize, Deserializer};
use std::time::Duration;

pub fn deserialize_optional_datetime_from_sec<'de, D>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt<T> {
        String(String),
        Number(T),
    }

    match StringOrInt::<u64>::deserialize(deserializer)? {
        StringOrInt::String(_s) => Ok(None),
        StringOrInt::Number(n) => {
            let duration = Duration::from_secs(n);
            Ok(Some(duration))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::serde_json_eq;

    use super::*;

    #[derive(Debug, Deserialize)]
    struct TestStruct {
        #[serde(default, deserialize_with = "deserialize_optional_datetime_from_sec")]
        pub timestamp: Option<Duration>,
        pub port: Option<i16>,
    }

    #[test]
    fn test_config_deserializes_duration() {
        serde_json_eq!(
            TestStruct,
            "{\"timestamp\":1234}",
            timestamp,
            Some(Duration::from_secs(1234))
        );
        serde_json_eq!(TestStruct, "{\"timestamp\":\"1234\"}", timestamp, None);
    }

    #[test]
    fn test_config_deserializes_port() {
        serde_json_eq!(TestStruct, "{\"port\":1234}", port, Some(1234));
    }
}
