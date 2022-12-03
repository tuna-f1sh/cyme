//! Colouring of cyme output
use colored::*;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// Colours [`Block`] fields based on loose typing of field type
///
/// Considered using HashMap with Colouring Enum like IconTheme but this seemed to suit better, it is less flexiable though...
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ColourTheme {
    /// Colour to use for name from descriptor
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub name: Option<Color>,
    /// Colour to use for serial from descriptor
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub serial: Option<Color>,
    /// Colour to use for manufacturer from descriptor
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub manufacturer: Option<Color>,
    /// Colour to use for driver from udev
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub driver: Option<Color>,
    /// Colour to use for general String data
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub string: Option<Color>,
    /// Colour to use for icons
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub icon: Option<Color>,
    /// Colour to use for location data
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub location: Option<Color>,
    /// Colour to use for path data
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub path: Option<Color>,
    /// Colour to use for general number values
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub number: Option<Color>,
    /// Colour to use for speed
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub speed: Option<Color>,
    /// Colour to use for Vendor ID
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub vid: Option<Color>,
    /// Colour to use for Product ID
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub pid: Option<Color>,
    /// Colour to use for generic ClassCode
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub class_code: Option<Color>,
    /// Colour to use for SubCodes
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub sub_code: Option<Color>,
    /// Colour to use for protocol
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub protocol: Option<Color>,
    /// Colour to use for info/enum type
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub attributes: Option<Color>,
    /// Colour to use for power information
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub power: Option<Color>,
    /// Tree colour
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub tree: Option<Color>,
    /// Colour at prepended before printing `USBBus`
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub tree_bus_start: Option<Color>,
    /// Colour printed at end of tree before printing `USBDevice`
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub tree_bus_terminator: Option<Color>,
    /// Colour printed at end of tree before printing configuration
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub tree_configuration_terminator: Option<Color>,
    /// Colour printed at end of tree before printing interface
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub tree_interface_terminator: Option<Color>,
    /// Colour for endpoint in before print
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub tree_endpoint_in: Option<Color>,
    /// Colour for endpoint out before print
    #[serde(
        default,
        serialize_with = "color_serializer",
        deserialize_with = "deserialize_option_color_from_string"
    )]
    pub tree_endpoint_out: Option<Color>,
}

fn deserialize_option_color_from_string<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumericOrNull<'a> {
        Str(&'a str),
        #[serde(deserialize_with = "deserialize_color")]
        FromStr(Color),
        Null,
    }

    match NumericOrNull::deserialize(deserializer)? {
        NumericOrNull::Str(s) => match s {
            "" => Ok(None),
            _ => Color::try_from(s)
                .map(Some)
                .map_err(serde::de::Error::custom),
        },
        NumericOrNull::FromStr(i) => Ok(Some(i)),
        NumericOrNull::Null => Ok(None),
    }
}

// Custom color deserialize, adapted from: https://github.com/Peltoche/lsd/blob/master/src/theme/color.rs
fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct ColorVisitor;
    impl<'de> serde::de::Visitor<'de> for ColorVisitor {
        type Value = Color;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("colour string or `3 u8 RGB array`")
        }

        fn visit_str<E>(self, value: &str) -> Result<Color, E>
        where
            E: serde::de::Error,
        {
            Color::try_from(value)
                .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(value), &self))
        }

        fn visit_seq<M>(self, mut seq: M) -> Result<Color, M::Error>
        where
            M: serde::de::SeqAccess<'de>,
        {
            let mut values = Vec::new();
            if let Some(size) = seq.size_hint() {
                if size != 3 {
                    return Err(serde::de::Error::invalid_length(
                        size,
                        &"a list of size 3(RGB)",
                    ));
                }
            }
            loop {
                match seq.next_element::<u8>() {
                    Ok(Some(x)) => {
                        values.push(x);
                    }
                    Ok(None) => break,
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            // recheck as size_hint sometimes not working
            if values.len() != 3 {
                return Err(serde::de::Error::invalid_length(
                    values.len(),
                    &"A u8 list of size 3: [R, G, B]",
                ));
            }
            Ok(Color::TrueColor {
                r: values[0],
                g: values[1],
                b: values[2],
            })
        }
    }

    deserializer.deserialize_any(ColorVisitor)
}

fn color_to_string(color: Color) -> String {
    match color {
        Color::Black => "black".into(),
        Color::Red => "red".into(),
        Color::Green => "green".into(),
        Color::Yellow => "yellow".into(),
        Color::Blue => "blue".into(),
        Color::Magenta => "magenta".into(),
        Color::Cyan => "cyan".into(),
        Color::White => "white".into(),
        Color::BrightBlack => "bright black".into(),
        Color::BrightRed => "bright red".into(),
        Color::BrightGreen => "bright green".into(),
        Color::BrightYellow => "bright yellow".into(),
        Color::BrightBlue => "bright blue".into(),
        Color::BrightMagenta => "bright magenta".into(),
        Color::BrightCyan => "bright cyan".into(),
        Color::BrightWhite => "bright white".into(),
        Color::TrueColor { r, g, b } => format!("[{}, {}, {}]", r, g, b),
    }
}

/// Have to make this because external crate does not impl Display
fn color_serializer<'a, S>(color: &'a Option<Color>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    match color {
        Some(c) => match c {
            Color::TrueColor { r, g, b } => {
                let mut seq = s.serialize_seq(Some(3))?;
                seq.serialize_element(r)?;
                seq.serialize_element(g)?;
                seq.serialize_element(b)?;
                seq.end()
            }
            _ => s.serialize_str(&color_to_string(*c)),
        },
        None => s.serialize_none(),
    }
}

impl Default for ColourTheme {
    fn default() -> Self {
        ColourTheme::new()
    }
}

impl ColourTheme {
    /// New theme with defaults
    pub fn new() -> Self {
        ColourTheme {
            name: Some(Color::BrightBlue),
            serial: Some(Color::Green),
            manufacturer: Some(Color::Blue),
            driver: Some(Color::Cyan),
            string: Some(Color::Blue),
            icon: None,
            location: Some(Color::Magenta),
            path: Some(Color::Cyan),
            number: Some(Color::Cyan),
            speed: Some(Color::Magenta),
            vid: Some(Color::BrightYellow),
            pid: Some(Color::Yellow),
            class_code: Some(Color::BrightYellow),
            sub_code: Some(Color::Yellow),
            protocol: Some(Color::Yellow),
            attributes: Some(Color::Magenta),
            power: Some(Color::Red),
            tree: Some(Color::BrightBlack),
            tree_bus_start: Some(Color::BrightBlack),
            tree_bus_terminator: Some(Color::BrightBlack),
            tree_configuration_terminator: Some(Color::BrightBlack),
            tree_interface_terminator: Some(Color::BrightBlack),
            tree_endpoint_in: Some(Color::Yellow),
            tree_endpoint_out: Some(Color::Magenta),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_color_theme() {
        let ct: ColourTheme = ColourTheme::new();
        println!("{}", serde_json::to_string_pretty(&ct).unwrap());
    }

    #[test]
    fn test_deserialize_color_theme() {
        let ct: ColourTheme = serde_json::from_str(r#"{"name": "blue"}"#).unwrap();
        assert_eq!(ct.name, Some(Color::Blue));
    }

    #[test]
    fn test_serialize_deserialize_color_theme() {
        let ct: ColourTheme = ColourTheme::new();
        let ser = serde_json::to_string_pretty(&ct).unwrap();
        let ctrt: ColourTheme = serde_json::from_str(&ser).unwrap();
        assert_eq!(ct, ctrt);
    }
}
