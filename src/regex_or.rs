use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug)]
pub enum RegexOr<T> {
    Regex(Regex),
    Other(T),
}

impl<T> Serialize for RegexOr<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            RegexOr::Regex(re) => serializer.serialize_str(&format!("/{}/", &re.to_string())),
            RegexOr::Other(o) => o.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for RegexOr<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_yaml::Value::deserialize(deserializer)?;
        if let serde_yaml::Value::String(ref s) = value {
            if s.starts_with("/") && s.ends_with("/") {
                let re_substr = &s[1..s.len() - 1];
                let re = re_substr.to_owned().try_into().map_err(D::Error::custom)?;
                return Ok(RegexOr::Regex(re));
            }
        }

        T::deserialize(value)
            .map(RegexOr::Other)
            .map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use serde_yaml;

    #[test]
    fn test_wild_or_ser() {
        // Test serializing the regex '*'
        let value: RegexOr<u32> = RegexOr::Regex(Regex::new(".").unwrap());
        let expected = "/./";
        let actual = serde_yaml::to_string(&value).unwrap();
        assert_eq!(expected, actual.trim());

        // Test serializing a vector
        let value: RegexOr<Vec<u32>> = RegexOr::Other(vec![1, 2, 3]);
        let expected = "- 1\n- 2\n- 3";
        let actual = serde_yaml::to_string(&value).unwrap();
        assert_eq!(expected, actual.trim());
    }
}
