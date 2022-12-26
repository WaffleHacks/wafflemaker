use serde::{
    de::{value::MapAccessDeserializer, Deserializer, Error, MapAccess, Unexpected, Visitor},
    Deserialize, Serialize,
};
use std::fmt::Formatter;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Secret {
    Aws {
        role: String,
        part: AwsPart,
    },
    Generate {
        format: Format,
        length: u32,
        regenerate: bool,
    },
    Load,
}

/// Which part of the AWS token pair to store in the variable
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AwsPart {
    Access,
    Secret,
}

/// The possible formats that a secret can be encoded into.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Alphanumeric,
    Base64,
    Hex,
}

impl Secret {
    /// Get the type of secret
    pub fn kind<'a>(&self) -> &'a str {
        match self {
            Secret::Aws { .. } => "aws",
            Secret::Generate { .. } => "generate",
            Secret::Load => "load",
        }
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SecretVisitor;

        impl<'de> Visitor<'de> for SecretVisitor {
            type Value = Secret;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "load" => Ok(Secret::Load),
                    _ => Err(Error::invalid_value(Unexpected::Str(v), &"one of 'load'")),
                }
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                /// Needed to avoid implementing the map deserializer ourselves
                #[derive(Deserialize)]
                #[serde(rename_all = "lowercase", tag = "type")]
                enum SecretMap {
                    Aws {
                        role: String,
                        part: AwsPart,
                    },
                    Generate {
                        format: Format,
                        length: u32,
                        #[serde(default)]
                        regenerate: bool,
                    },
                    Load,
                }

                let map: SecretMap = Deserialize::deserialize(MapAccessDeserializer::new(map))?;
                Ok(match map {
                    SecretMap::Aws { role, part } => Secret::Aws { role, part },
                    SecretMap::Generate {
                        format,
                        length,
                        regenerate,
                    } => Secret::Generate {
                        format,
                        length,
                        regenerate,
                    },
                    SecretMap::Load => Secret::Load,
                })
            }
        }

        deserializer.deserialize_any(SecretVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, Secret};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Wrapper {
        secret: Secret,
    }

    #[test]
    fn deserialize_string() {
        let src = r#"
        secret = "load"
        "#;
        let parsed = toml::from_str::<Wrapper>(src).unwrap();

        assert_eq!(parsed.secret, Secret::Load);
    }

    #[test]
    fn deserialize_non_string_type() {
        let src = r#"
        secret = "aws"
        "#;
        let parsed = toml::from_str::<Wrapper>(src).unwrap_err();

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
        let parsed = toml::from_str::<Wrapper>(src).unwrap();

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
