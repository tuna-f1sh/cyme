//! Types used in crate non-specific to a module
use std::fmt;
use std::str::FromStr;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::error::{self, Error, ErrorKind};

/// A vendor and product ID pair used in filter expressions, serialised as a hex string.
///
/// Accepts the formats `"VID:PID"`, `"VID:"`, `"VID"` (PID wildcard), or `":PID"` (VID wildcard).
/// Both components are optional; an absent component matches any value.
///
/// ```
/// use cyme::types::VidPid;
/// let vp: VidPid = "1d50:6018".parse().unwrap();
/// assert_eq!(vp, VidPid(Some(0x1d50), Some(0x6018)));
/// let vp: VidPid = "05ac".parse().unwrap();
/// assert_eq!(vp, VidPid(Some(0x05ac), None));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, DeserializeFromStr, SerializeDisplay)]
pub struct VidPid(pub Option<u16>, pub Option<u16>);

impl fmt::Display for VidPid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.0, self.1) {
            (Some(vid), Some(pid)) => write!(f, "{vid:04x}:{pid:04x}"),
            (Some(vid), None) => write!(f, "{vid:04x}"),
            (None, Some(pid)) => write!(f, ":{pid:04x}"),
            (None, None) => write!(f, ":"),
        }
    }
}

impl FromStr for VidPid {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        fn parse_hex(s: &str) -> error::Result<u16> {
            u16::from_str_radix(s.trim().trim_start_matches("0x"), 16)
                .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))
        }
        if let Some((vid_str, pid_str)) = s.split_once(':') {
            let vid = if vid_str.trim().is_empty() {
                None
            } else {
                Some(parse_hex(vid_str)?)
            };
            let pid = if pid_str.trim().is_empty() {
                None
            } else {
                Some(parse_hex(pid_str)?)
            };
            Ok(VidPid(vid, pid))
        } else {
            Ok(VidPid(Some(parse_hex(s)?), None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vidpid_parse() {
        assert_eq!(
            "000A:0x000b".parse::<VidPid>().unwrap(),
            VidPid(Some(0x0A), Some(0x0b))
        );
        assert_eq!(
            "000A:1".parse::<VidPid>().unwrap(),
            VidPid(Some(0x0A), Some(1))
        );
        assert_eq!("000A:".parse::<VidPid>().unwrap(), VidPid(Some(0x0A), None));
        assert_eq!(
            "0x000A".parse::<VidPid>().unwrap(),
            VidPid(Some(0x0A), None)
        );
        assert!("dfg:sdfd".parse::<VidPid>().is_err());
        // values that would silently truncate with u32-as-u16 should now error
        assert!("1ffff".parse::<VidPid>().is_err());
    }
}

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
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct NumericalUnit<T> {
    /// Numerical value
    pub value: T,
    /// SI unit for the numerical value
    pub unit: String,
    /// Description of numerical value
    pub description: Option<String>,
}

impl fmt::Display for NumericalUnit<u32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(width) = f.width() {
            let actual_width = width - self.unit.len() - 1;
            write!(f, "{:actual_width$} {:}", self.value, self.unit)
        } else {
            write!(f, "{:} {:}", self.value, self.unit)
        }
    }
}

impl fmt::Display for NumericalUnit<f32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // If we received a precision, we use it.
        if let Some(width) = f.width() {
            let actual_width = width - self.unit.len() - 1;
            write!(
                f,
                "{1:actual_width$.*} {2}",
                f.precision().unwrap_or(2),
                self.value,
                self.unit
            )
        } else {
            write!(
                f,
                "{1:.*} {2}",
                f.precision().unwrap_or(2),
                self.value,
                self.unit
            )
        }
    }
}

impl FromStr for NumericalUnit<u32> {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        let value_split: Vec<&str> = s.trim().split(' ').collect();
        if value_split.len() >= 2 {
            Ok(NumericalUnit {
                value: value_split[0]
                    .trim()
                    .parse::<u32>()
                    .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))?,
                unit: value_split[1].trim().to_string(),
                description: None,
            })
        } else {
            Err(Error::new(
                ErrorKind::Decoding,
                "string split does not contain [u32] [unit]",
            ))
        }
    }
}

impl FromStr for NumericalUnit<f32> {
    type Err = Error;

    fn from_str(s: &str) -> error::Result<Self> {
        let value_split: Vec<&str> = s.trim().split(' ').collect();
        if value_split.len() >= 2 {
            Ok(NumericalUnit {
                value: value_split[0]
                    .trim()
                    .parse::<f32>()
                    .map_err(|e| Error::new(ErrorKind::Parsing, &e.to_string()))?,
                unit: value_split[1].trim().to_string(),
                description: None,
            })
        } else {
            Err(Error::new(
                ErrorKind::Decoding,
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
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        #[serde(untagged)]
        enum Field {
            Value,
            Unit,
            Description,
        }

        struct DeviceNumericalUnitU32Visitor;

        impl<'de> Visitor<'de> for DeviceNumericalUnitU32Visitor {
            type Value = NumericalUnit<u32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with format '[int] [unit]'")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<NumericalUnit<u32>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let value = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let unit = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let description = seq
                    .next_element()
                    .ok()
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                Ok(NumericalUnit::<u32> {
                    value,
                    unit,
                    description,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<NumericalUnit<u32>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut value = None;
                let mut unit = None;
                let mut description = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Value => {
                            if value.is_some() {
                                return Err(de::Error::duplicate_field("value"));
                            }
                            value = Some(map.next_value()?);
                        }
                        Field::Unit => {
                            if unit.is_some() {
                                return Err(de::Error::duplicate_field("unit"));
                            }
                            unit = Some(map.next_value()?);
                        }
                        Field::Description => {
                            if description.is_some() {
                                return Err(de::Error::duplicate_field("description"));
                            }
                            description = map.next_value().ok();
                        }
                    }
                }
                let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                let unit = unit.ok_or_else(|| de::Error::missing_field("unit"))?;
                Ok(NumericalUnit::<u32> {
                    value,
                    unit,
                    description,
                })
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                NumericalUnit::from_str(value.as_str()).map_err(E::custom)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                NumericalUnit::from_str(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(DeviceNumericalUnitU32Visitor)
    }
}

impl<'de> Deserialize<'de> for NumericalUnit<f32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        #[serde(untagged)]
        enum Field {
            Value,
            Unit,
            #[serde(deserialize_with = "deserialize_description")]
            Description,
        }

        struct DeviceNumericalUnitF32Visitor;

        impl<'de> Visitor<'de> for DeviceNumericalUnitF32Visitor {
            type Value = NumericalUnit<f32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with format '[float] [unit]'")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<NumericalUnit<f32>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let value = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let unit = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let description = seq
                    .next_element()
                    .ok()
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                Ok(NumericalUnit::<f32> {
                    value,
                    unit,
                    description,
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<NumericalUnit<f32>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut value = None;
                let mut unit = None;
                let mut description = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Value => {
                            if value.is_some() {
                                return Err(de::Error::duplicate_field("value"));
                            }
                            value = Some(map.next_value()?);
                        }
                        Field::Unit => {
                            if unit.is_some() {
                                return Err(de::Error::duplicate_field("unit"));
                            }
                            unit = Some(map.next_value()?);
                        }
                        Field::Description => {
                            if description.is_some() {
                                return Err(de::Error::duplicate_field("description"));
                            }
                            description = map.next_value().ok();
                        }
                    }
                }
                let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                let unit = unit.ok_or_else(|| de::Error::missing_field("unit"))?;
                Ok(NumericalUnit::<f32> {
                    value,
                    unit,
                    description,
                })
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                NumericalUnit::from_str(value.as_str()).map_err(E::custom)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                NumericalUnit::from_str(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(DeviceNumericalUnitF32Visitor)
    }
}
