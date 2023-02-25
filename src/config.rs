//! Config for cyme binary
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::colour;
use crate::display;
use crate::display::Block;
use crate::icon;

const CONF_DIR: &'static str = "cyme";
const CONF_NAME: &'static str = "cyme.json";

/// Allows user supplied icons to replace or add to `DEFAULT_ICONS` and `DEFAULT_TREE`
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct Config {
    /// User supplied [`crate::icon::IconTheme`] - will merge with default
    pub icons: icon::IconTheme,
    /// User supplied [`crate::colour::ColourTheme`] - overrides default
    pub colours: colour::ColourTheme,
    /// Default [`crate::display::DeviceBlocks`] to use for displaying devices
    pub blocks: Option<Vec<display::DeviceBlocks>>,
    /// Default [`crate::display::BusBlocks`] to use for displaying buses
    pub bus_blocks: Option<Vec<display::BusBlocks>>,
    /// Default [`crate::display::ConfigurationBlocks`] to use for device configurations
    pub config_blocks: Option<Vec<display::ConfigurationBlocks>>,
    /// Default [`crate::display::InterfaceBlocks`] to use for device interfaces
    pub interface_blocks: Option<Vec<display::InterfaceBlocks>>,
    /// Default [`crate::display::EndpointBlocks`] to use for device endpoints
    pub endpoint_blocks: Option<Vec<display::EndpointBlocks>>,
    /// Wether to hide device serial numbers by default
    pub mask_serials: Option<display::MaskSerial>,
    /// Max variable string length to display before truncating - descriptors and classes for example
    pub max_variable_string_len: Option<usize>,
    /// Disable auto generation of max_variable_string_len based on terminal width
    pub no_auto_width: bool,
    // non-Options copied from Args
    /// Attempt to maintain compatibility with lsusb output
    pub lsusb: bool,
    /// Dump USB device hierarchy as a tree
    pub tree: bool,
    /// Verbosity level: 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and all blocks
    pub verbose: u8,
    /// Print more blocks by default at each verbosity
    pub more: bool,
    /// Hide empty buses when printing tree; those with no devices. When listing will hide Linux root_hubs
    pub hide_buses: bool,
    /// Hide empty hubs when printing tree; those with no devices. When listing will hide hubs regardless of whether empty of not
    pub hide_hubs: bool,
    /// Show base16 values as base10 decimal instead
    pub decimal: bool,
    /// Disable padding to align blocks
    pub no_padding: bool,
    /// Disables icons and utf-8 charactors
    pub ascii: bool,
    /// Show block headings
    pub headings: bool,
    /// Force libusb profiler on macOS rather than using/combining system_profiler output
    pub force_libusb: bool,
}

impl Config {
    /// New based on defaults
    pub fn new() -> Config {
        Default::default()
    }

    /// From system config if exists else default
    #[cfg(not(debug_assertions))]
    pub fn sys() -> Result<Config, io::Error> {
        if let Some(p) = Self::config_file_path() {
            let path = p.join(CONF_NAME);
            log::info!("Looking for cyme system config {:?}", &path);
            return match Self::from_file(&path) {
                Ok(c) => {
                    log::info!("Loaded cyme system config {:?}", c);
                    Ok(c)
                }
                Err(e) => {
                    // only return error it's not found as use default in that case
                    if e.kind() != io::ErrorKind::NotFound {
                        log::warn!(
                            "Failed to read cyme system config {:?}: Error({})",
                            &path,
                            e
                        );
                        Err(e)
                    } else {
                        Ok(Self::new())
                    }
                }
            };
        } else {
            Ok(Self::new())
        }
    }

    /// Use default if running in debug since the integration tests use this
    #[cfg(debug_assertions)]
    pub fn sys() -> Result<Config, io::Error> {
        log::warn!("Running in debug, not checking for cyme system config");
        Ok(Self::new())
    }

    /// Get example [`Config`]
    pub fn example() -> Config {
        Config {
            icons: icon::example_theme(),
            blocks: Some(display::DeviceBlocks::default_blocks(false)),
            bus_blocks: Some(display::BusBlocks::default_blocks(false)),
            config_blocks: Some(display::ConfigurationBlocks::default_blocks(false)),
            interface_blocks: Some(display::InterfaceBlocks::default_blocks(false)),
            endpoint_blocks: Some(display::EndpointBlocks::default_blocks(false)),
            ..Default::default()
        }
    }

    /// Attempt to read from .json format confg at `file_path`
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Config, io::Error> {
        let f = File::open(file_path)?;
        let mut br = BufReader::new(f);
        let mut data = String::new();

        br.read_to_string(&mut data)?;
        serde_json::from_str::<Config>(&data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    /// This provides the path for a configuration file, specific to OS
    /// return None if error like PermissionDenied
    pub fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|x| x.join(CONF_DIR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_example_file() {
        let path = PathBuf::from("./doc").join("cyme_example_config.json");
        assert_eq!(Config::from_file(path).is_ok(), true);
    }

    #[test]
    fn test_deserialize_config_no_theme() {
        let path = PathBuf::from("./tests/data").join("config_no_theme.json");
        assert_eq!(Config::from_file(path).is_ok(), true);
    }

    #[test]
    fn test_deserialize_config_missing_args() {
        let path = PathBuf::from("./tests/data").join("config_missing_args.json");
        assert_eq!(Config::from_file(path).is_ok(), true);
    }
}
