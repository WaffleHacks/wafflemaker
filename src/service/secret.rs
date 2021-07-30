use serde::{
    de::{value::MapAccessDeserializer, Deserializer, Error, MapAccess, Unexpected, Visitor},
    Deserialize, Serialize,
};
use std::{fmt, str::FromStr};

/// The possible formats that a secret can be encoded into.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Alphanumeric,
    Base64,
    Hex,
}

/// Which part of the pair to store in the variable
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Part {
    Access,
    Secret,
}

/// The possible secret types that can be retrieved/generated.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Secret {
    Aws {
        role: String,
        part: Part,
    },
    Generate {
        format: Format,
        length: u32,
        regenerate: bool,
    },
    Load,
}

impl Secret {
    /// Get the type of secret
    pub fn name<'a>(&self) -> &'a str {
        match self {
            Secret::Aws { .. } => "aws",
            Secret::Generate { .. } => "generate",
            Secret::Load => "load",
        }
    }
}

impl From<AuxiliarySecret> for Secret {
    fn from(aux: AuxiliarySecret) -> Secret {
        match aux {
            AuxiliarySecret::Aws { role, part } => Secret::Aws { role, part },
            AuxiliarySecret::Generate {
                format,
                length,
                regenerate,
            } => Secret::Generate {
                format,
                length,
                regenerate,
            },
            AuxiliarySecret::Load => Secret::Load,
        }
    }
}

/// `AuxiliarySecret` exists to avoid implementing the deserializer of the map by hand which
/// means we cannot use `Secret` itself as it would cause infinite recursion.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
enum AuxiliarySecret {
    Aws {
        role: String,
        part: Part,
    },
    Generate {
        format: Format,
        length: u32,
        #[serde(default)]
        regenerate: bool,
    },
    Load,
}

struct SecretVisitor;

impl<'de> Visitor<'de> for SecretVisitor {
    type Value = Secret;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        FromStr::from_str(value)
            .map_err(|m: String| Error::invalid_value(Unexpected::Str(value), &m.as_str()))
    }

    fn visit_map<M>(self, visitor: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let aux: AuxiliarySecret = Deserialize::deserialize(MapAccessDeserializer::new(visitor))?;
        Ok(aux.into())
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Secret, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SecretVisitor)
    }
}

impl FromStr for Secret {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "load" => Ok(Self::Load),
            _ => Err("one of 'load'".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;
    use crate::service::Format;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Wrapped {
        secret: Secret,
    }

    #[test]
    fn deserialize_string() {
        let src = r#"
        secret = "load"
        "#;
        let parsed: Wrapped = toml::from_str(src).unwrap();

        assert_eq!(parsed.secret, Secret::Load);
    }

    #[test]
    fn deserialize_invalid_string() {
        let src = r#"
        secret = "aws"
        "#;
        let parsed = toml::from_str::<Wrapped>(src).unwrap_err();

        assert_eq!(
            parsed.to_string().as_str(),
            "invalid value: string \"aws\", expected one of \'load\' for key `secret` at line 2 column 18"
        );
    }

    #[test]
    fn deserialize_map() {
        let src = r#"
        [secret]
        type = "generate"
        length = 16
        format = "base64"
        regenerate = true
        "#;
        let parsed: Wrapped = toml::from_str(src).unwrap();

        assert!(matches!(parsed.secret, Secret::Generate { .. }));
        if let Secret::Generate {
            length,
            format,
            regenerate,
        } = parsed.secret
        {
            assert_eq!(length, 16);
            assert_eq!(format, Format::Base64);
            assert_eq!(regenerate, true);
        }
    }
}
