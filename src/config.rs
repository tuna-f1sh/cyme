//! Config for cyme binary
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::colour;
use crate::display;
use crate::display::Block;
use crate::error::{Error, ErrorKind, Result};
use crate::icon;

const CONF_DIR: &str = "cyme";
const CONF_NAME: &str = "cyme.json";

/// Allows user supplied icons to replace or add to `DEFAULT_ICONS` and `DEFAULT_TREE`
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct Config {
    #[serde(skip)]
    filepath: Option<PathBuf>,
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
    /// Whether to hide device serial numbers by default
    pub mask_serials: Option<display::MaskSerial>,
    /// How to group devices during display
    pub group_devices: Option<display::Group>,
    /// Encoding to use for output text
    pub encoding: Option<display::Encoding>,
    /// When to show icons
    pub icon_when: Option<display::IconWhen>,
    /// When to use color
    pub color_when: Option<display::ColorWhen>,
    /// How to sort devices when listing
    pub sort_devices: Option<display::Sort>,
    /// Sort devices by bus number - irrelevant unless sort_devices is NoSort
    pub sort_buses: bool,
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
    /// Hide empty buses when printing tree; those with no devices.
    pub hide_buses: bool,
    /// Hide empty hubs when printing tree; those with no devices. When listing will hide hubs regardless of whether empty of not
    pub hide_hubs: bool,
    /// Show root hubs when listing; Linux only
    pub list_root_hubs: bool,
    /// Show base16 values as base10 decimal instead
    pub decimal: bool,
    /// Disable padding to align blocks
    pub no_padding: bool,
    /// Disable color - depreciated use color_when
    #[serde(skip_serializing)]
    pub no_color: bool,
    /// Disables icons and utf-8 characters - depreciated use encoding
    #[serde(skip_serializing)]
    pub ascii: bool,
    /// Disables all [`display::Block`] icons - depreciated use icon_when
    #[serde(skip_serializing)]
    pub no_icons: bool,
    /// Show block headings
    pub headings: bool,
    /// Force nusb/libusb profiler on macOS rather than using/combining system_profiler output
    pub force_libusb: bool,
    /// Output in JSON format
    pub json: bool,
    /// Print non-critical errors (normally due to permissions) during USB profiler to stderr
    pub print_non_critical_profiler_stderr: bool,
}

impl Config {
    /// New based on defaults
    pub fn new() -> Self {
        Default::default()
    }

    /// From system config if exists else default
    #[cfg(not(debug_assertions))]
    pub fn sys() -> Result<Self> {
        if let Some(p) = Self::config_file_path() {
            let path = p.join(CONF_NAME);
            log::info!("Looking for system config {:?}", &path);
            return match Self::from_file(&path) {
                Ok(c) => {
                    log::info!("Loaded system config {:?}", c);
                    Ok(c)
                }
                Err(e) => {
                    // if parsing error, print issue but use default
                    // IO error (unable to read) will raise as error
                    if e.kind() == ErrorKind::Parsing {
                        log::warn!("{}", e);
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
    pub fn sys() -> Result<Self> {
        log::warn!("Running in debug, not checking for system config");
        Ok(Self::new())
    }

    /// Get example [`Config`]
    pub fn example() -> Self {
        Config {
            icons: icon::example_theme(),
            blocks: Some(display::DeviceBlocks::example_blocks()),
            bus_blocks: Some(display::BusBlocks::example_blocks()),
            config_blocks: Some(display::ConfigurationBlocks::example_blocks()),
            interface_blocks: Some(display::InterfaceBlocks::example_blocks()),
            endpoint_blocks: Some(display::EndpointBlocks::example_blocks()),
            mask_serials: None,
            group_devices: Some(display::Group::default()),
            encoding: Some(display::Encoding::default()),
            icon_when: Some(display::IconWhen::default()),
            color_when: Some(display::ColorWhen::default()),
            sort_devices: Some(display::Sort::default()),
            ..Default::default()
        }
    }

    /// Attempt to read from .json format config at `file_path`
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let f = File::open(&file_path)?;
        let mut config: Self = serde_json::from_reader(BufReader::new(f)).map_err(|e| {
            Error::new(
                ErrorKind::Parsing,
                &format!(
                    "Failed to parse config at {:?}; Error({})",
                    file_path.as_ref(),
                    e
                ),
            )
        })?;
        // set the file path we loaded from for saving
        config.filepath = Some(file_path.as_ref().to_path_buf());
        Ok(config)
    }

    /// This provides the path for a configuration file, specific to OS
    /// return None if error like PermissionDenied
    pub fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|x| x.join(CONF_DIR))
    }

    /// Get the file path for the config
    pub fn filepath(&self) -> Option<&Path> {
        self.filepath.as_deref()
    }

    /// Save the current config to a file
    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        log::info!("Saving config to {:?}", path.as_ref().display());
        // create parent folders
        if let Some(parent) = path.as_ref().parent() {
            log::debug!("Creating parent folders for {:?}", parent.display());
            std::fs::create_dir_all(parent)?;
        }
        let f = File::create(&path)?;
        serde_json::to_writer_pretty(f, self)
            .map_err(|e| Error::new(ErrorKind::Io, &format!("Failed to save config: Error({e})")))
    }

    /// Save the current config to the file it was loaded from or default location if None
    pub fn save(&self) -> Result<()> {
        if let Some(p) = self.filepath() {
            self.save_file(p)
        } else if let Some(p) = Self::config_file_path() {
            self.save_file(p.join(CONF_NAME))
        } else {
            Err(Error::new(
                ErrorKind::Io,
                "Unable to determine config file path",
            ))
        }
    }

    /// Merge the settings from a [`display::PrintSettings`] into the config
    ///
    /// Dynamic settings and those loaded from config such as [`icon::IconTheme`] and [`color::ColourTheme`] are not merged
    pub fn merge_print_settings(&mut self, settings: &display::PrintSettings) {
        self.blocks = settings.device_blocks.clone();
        self.bus_blocks = settings.bus_blocks.clone();
        self.config_blocks = settings.config_blocks.clone();
        self.interface_blocks = settings.interface_blocks.clone();
        self.endpoint_blocks = settings.endpoint_blocks.clone();
        self.more = settings.more;
        self.decimal = settings.decimal;
        self.mask_serials = settings.mask_serials;
        self.group_devices = Some(settings.group_devices);
        self.encoding = Some(settings.encoding);
        self.icon_when = Some(settings.icon_when);
        self.color_when = Some(settings.color_when);
        self.sort_devices = Some(settings.sort_devices);
        self.sort_buses = settings.sort_buses;
        self.no_color = settings.colours.is_none();
        self.no_padding = settings.no_padding;
        self.headings = settings.headings;
        self.tree = settings.tree;
        self.max_variable_string_len = settings.max_variable_string_len;
        self.no_auto_width = !settings.auto_width;
        self.no_icons = matches!(settings.icon_when, display::IconWhen::Never)
            || !matches!(settings.encoding, display::Encoding::Glyphs);
        self.ascii = matches!(settings.encoding, display::Encoding::Ascii);
        self.verbose = settings.verbosity;
        self.json = settings.json;
    }

    /// Returns a [`display::PrintSettings`] based on the config
    pub fn print_settings(&self) -> display::PrintSettings {
        let colours = if self.no_color {
            None
        } else {
            Some(self.colours.clone())
        };
        let icons = if self.no_icons {
            None
        } else {
            Some(self.icons.clone())
        };
        let encoding = self.encoding.unwrap_or({
            if self.ascii {
                display::Encoding::Ascii
            } else if self.no_icons {
                display::Encoding::Utf8
            } else {
                display::Encoding::Glyphs
            }
        });
        let group_devices = if self.group_devices == Some(display::Group::Bus) && self.tree {
            log::warn!("--group-devices with --tree is ignored; will print as tree");
            display::Group::NoGroup
        } else {
            self.group_devices.unwrap_or(display::Group::NoGroup)
        };
        display::PrintSettings {
            device_blocks: self.blocks.clone(),
            bus_blocks: self.bus_blocks.clone(),
            config_blocks: self.config_blocks.clone(),
            interface_blocks: self.interface_blocks.clone(),
            endpoint_blocks: self.endpoint_blocks.clone(),
            more: self.more,
            decimal: self.decimal,
            mask_serials: self.mask_serials,
            group_devices,
            sort_devices: self.sort_devices.unwrap_or_default(),
            sort_buses: self.sort_buses,
            no_padding: self.no_padding,
            headings: self.headings,
            tree: self.tree,
            max_variable_string_len: self.max_variable_string_len,
            auto_width: !self.no_auto_width,
            icon_when: self.icon_when.unwrap_or_default(),
            color_when: self.color_when.unwrap_or_default(),
            encoding,
            icons,
            colours,
            verbosity: self.verbose,
            json: self.json,
            ..Default::default()
        }
    }
}

impl From<&display::PrintSettings> for Config {
    fn from(settings: &display::PrintSettings) -> Self {
        let mut c = Config::new();
        c.merge_print_settings(settings);
        c
    }
}

impl From<&Config> for display::PrintSettings {
    fn from(c: &Config) -> Self {
        c.print_settings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "regex_icon")]
    fn test_deserialize_example_file() {
        let path = PathBuf::from("./doc").join("cyme_example_config.json");
        assert!(Config::from_file(path).is_ok());
    }

    #[test]
    fn test_deserialize_config_no_theme() {
        let path = PathBuf::from("./tests/data").join("config_no_theme.json");
        assert!(Config::from_file(path).is_ok());
    }

    #[test]
    fn test_deserialize_config_missing_args() {
        let path = PathBuf::from("./tests/data").join("config_missing_args.json");
        assert!(Config::from_file(path).is_ok());
    }

    #[test]
    fn test_save_config() {
        // save to temp file
        let path = PathBuf::from("./tests/data").join("config_save.json");
        let c = Config::new();
        assert!(c.save_file(&path).is_ok());
        assert!(Config::from_file(path).is_ok());
    }
}
