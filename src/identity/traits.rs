use std::fmt::{
    Display,
    Formatter,
};
use std::str::FromStr;

use serde::{
    Deserialize,
    Serialize,
};

use super::{
    TaskId,
    TaskIdentityError,
    TaskNodeId,
    TaskTreeId,
};
use crate::scheme::UrnScheme;

macro_rules! impl_string_traits {
    ($type:ty, $parse:path) => {
        impl Display for $type {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $type {
            type Err = TaskIdentityError;

            fn from_str(raw: &str) -> Result<Self, Self::Err> {
                $parse(raw)
            }
        }

        impl AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                $parse(&raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

macro_rules! impl_scheme_string_traits {
    ($type:ident, $parse:path) => {
        impl<S: UrnScheme> Display for $type<S> {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl<S: UrnScheme> FromStr for $type<S> {
            type Err = TaskIdentityError;

            fn from_str(raw: &str) -> Result<Self, Self::Err> {
                $parse(raw)
            }
        }

        impl<S: UrnScheme> AsRef<str> for $type<S> {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl<S: UrnScheme> Serialize for $type<S> {
            fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
            where
                Ser: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de, S: UrnScheme> Deserialize<'de> for $type<S> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                $parse(&raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

impl_scheme_string_traits!(TaskId, TaskId::parse);
impl_scheme_string_traits!(TaskTreeId, TaskTreeId::new);
impl_string_traits!(TaskNodeId, TaskNodeId::new);
