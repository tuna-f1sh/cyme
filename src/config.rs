//! Config for cyme binary
use std::io;
use std::fs::File;
use std::io::{BufReader, Read};
use serde::{Deserialize, Serialize};

use crate::icon;
use crate::colour;

/// Allows user supplied icons to replace or add to `DEFAULT_ICONS` and `DEFAULT_TREE`
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// User supplied [`IconTheme`] - will merge with default
    pub icons: icon::IconTheme,
    /// User supplied [`ColourTheme`] - overrides default
    pub colours: colour::ColourTheme,
}

impl Config {
    /// Default new
    pub fn new() -> Config {
        Config {
            ..Default::default()
        }
    }

    /// Get example [`Config`]
    pub fn example() -> Config {
        Config {
            icons: icon::example_theme(),
            ..Default::default()
        }
    }

    /// Attempt to read from .json format confg at `file_path`
    pub fn from_file(file_path: &str) -> Result<Config, io::Error> {
        let f = File::open(file_path)?;
        let mut br = BufReader::new(f);
        let mut data = String::new();

        br.read_to_string(&mut data)?;
        serde_json::from_str::<Config>(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
