//! Colouring of cyme output
use colored::*;
use std::fmt;
// use serde::de::{self, Visitor, MapAccess, SeqAccess};
// use serde::{Deserialize, Deserializer, Serialize};
// use std::str::FromStr;

/// Colours [`Block`] fields based on loose typing of field type
///
/// Considered using HashMap with Colouring Enum like IconTheme but this seemed to suit better, it is less flexiable though...
#[derive(Debug)]
pub struct ColourTheme {
    /// Colour to use for name from descriptor
    // #[serde(default)]
    // #[serde(deserialize_with = "deserialize_color")]
    pub name: Option<Color>,
    /// Colour to use for serial from descriptor
    pub serial: Option<Color>,
    /// Colour to use for manufacturer from descriptor
    pub manufacturer: Option<Color>,
    /// Colour to use for driver from udev
    pub driver: Option<Color>,
    /// Colour to use for general String data
    pub string: Option<Color>,
    /// Colour to use for icons
    pub icon: Option<Color>,
    /// Colour to use for location data
    pub location: Option<Color>,
    /// Colour to use for path data
    pub path: Option<Color>,
    /// Colour to use for general number values
    pub number: Option<Color>,
    /// Colour to use for speed
    pub speed: Option<Color>,
    /// Colour to use for Vendor ID
    pub vid: Option<Color>,
    /// Colour to use for Product ID
    pub pid: Option<Color>,
    /// Colour to use for generic ClassCode
    pub class_code: Option<Color>,
    /// Colour to use for SubCodes
    pub sub_code: Option<Color>,
    /// Colour to use for protocol
    pub protocol: Option<Color>,
    /// Colour to use for info/enum type
    pub attributes: Option<Color>,
    /// Colour to use for power information
    pub power: Option<Color>,
    /// Tree colour
    pub tree: Option<Color>,
    /// Colour at prepended before printing `USBBus`
    pub tree_bus_start: Option<Color>,
    /// Colour printed at end of tree before printing `USBDevice`
    pub tree_bus_terminator: Option<Color>,
    /// Colour printed at end of tree before printing configuration
    pub tree_configuration_terminator: Option<Color>,
    /// Colour printed at end of tree before printing interface
    pub tree_interface_terminator: Option<Color>,
    /// Colour for endpoint in before print
    pub tree_endpoint_in: Option<Color>,
    /// Colour for endpoint out before print
    pub tree_endpoint_out: Option<Color>,
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
            formatter.write_str(
                "colour string or `3 u8 RGB array`",
            )
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
            Ok(Color::TrueColor{r: values[0], g: values[1], b: values[2]})
        }
    }

    deserializer.deserialize_any(ColorVisitor)
}

impl ColourTheme {
    /// New theme with defaults
    pub fn new() -> Self {
        ColourTheme{
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_serialize_color_value() {
//         let color_value = ColorValue{ value: Color::Black };
//         println!("{}", serde_json::to_string_pretty(&color_value).unwrap());
//     }
// }
