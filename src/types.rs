//! Types used in crate non-specific to a module
use std::fmt;
use std::io;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// A numerical `value` converted from a String, which includes a `unit` and `description`
///
/// Serialized string is of format "\[value\] \[unit\]" where u32 of f32 is supported
///
/// ```
/// use std::str::FromStr;
/// use cyme::types::NumericalUnit;
///
/// let s: &'static str = "100.0 W";
/// let nu = NumericalUnit::from_str(s).unwrap();
/// assert_eq!(nu, NumericalUnit{ value: 100.0, unit: "W".into(), description: None });
///
/// let s: &'static str = "59 mA";
/// let nu = NumericalUnit::from_str(s).unwrap();
/// assert_eq!(nu, NumericalUnit{ value: 59, unit: "mA".into(), description: None });
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NumericalUnit<T> {
    pub value: T,
    pub unit: String,
    pub description: Option<String>,
}

impl fmt::Display for NumericalUnit<u32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:} {:}", self.value, self.unit)
    }
}

impl fmt::Display for NumericalUnit<f32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // If we received a precision, we use it.
        write!(
            f,
            "{1:.*} {2}",
            f.precision().unwrap_or(2),
            self.value,
            self.unit
        )
    }
}

impl FromStr for NumericalUnit<u32> {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value_split: Vec<&str> = s.trim().split(' ').collect();
        if value_split.len() >= 2 {
            Ok(NumericalUnit {
                value: value_split[0]
                    .trim()
                    .parse::<u32>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                unit: value_split[1].trim().to_string(),
                description: None,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "string split does not contain [u32] [unit]",
            ))
        }
    }
}

impl FromStr for NumericalUnit<f32> {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value_split: Vec<&str> = s.trim().split(' ').collect();
        if value_split.len() >= 2 {
            Ok(NumericalUnit {
                value: value_split[0]
                    .trim()
                    .parse::<f32>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                unit: value_split[1].trim().to_string(),
                description: None,
            })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "string split does not contain [f32] [unit]",
            ))
        }
    }
}

impl<'de> Deserialize<'de> for NumericalUnit<u32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DeviceNumericalUnitU32Visitor;

        impl<'de> Visitor<'de> for DeviceNumericalUnitU32Visitor {
            type Value = NumericalUnit<u32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with format '[int] [unit]'")
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value.as_str()).map_err(E::custom)?)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value).map_err(E::custom)?)
            }
        }

        deserializer.deserialize_str(DeviceNumericalUnitU32Visitor)
    }
}

impl<'de> Deserialize<'de> for NumericalUnit<f32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DeviceNumericalUnitF32Visitor;

        impl<'de> Visitor<'de> for DeviceNumericalUnitF32Visitor {
            type Value = NumericalUnit<f32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with format '[float] [unit]'")
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value.as_str()).map_err(E::custom)?)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NumericalUnit::from_str(value).map_err(E::custom)?)
            }
        }

        deserializer.deserialize_str(DeviceNumericalUnitF32Visitor)
    }
}
