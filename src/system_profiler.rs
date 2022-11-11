/// Parser for macOS `system_profiler` command -json output with SPUSBDataType.
///
/// USBBus and USBDevice structs are used as deserializers for serde. The JSON output with the -json flag is not really JSON; all values are String regardless of contained data so it requires some extra work. Additionally, some values differ slightly from the non json output such as the speed - it is a description rather than numerical.
///
/// J.Whittington - 2022
use std::io;
use std::fmt;
use std::str::FromStr;

use colored::*;
use std::process::Command;
use serde::{Deserialize, Deserializer, Serialize};

/// borrowed from https://github.com/vityafx/serde-aux/blob/master/src/field_attributes.rs with addition of base16 encoding
/// Deserializes an option number from string or a number.
fn deserialize_option_number_from_string<'de, T, D>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: fmt::Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumericOrNull<'a, T> {
        Str(&'a str),
        FromStr(T),
        Null,
    }

    match NumericOrNull::<T>::deserialize(deserializer)? {
        NumericOrNull::Str(mut s) => match s {
            "" => Ok(None),
            _ => {
                // -json returns apple_vendor_id in vendor_id for some reason not base16 like normal
                if s == "apple_vendor_id" {
                    s = "0x05ac";
                }
                // the vendor_id can be appended with manufacturer name for some reason...split with space to get just base16 encoding
                let vendor_vec: Vec<&str> = s.split(" ").collect();
                let removed_0x = vendor_vec[0].trim_start_matches("0x");

                if removed_0x != s {
                    let base16_num = u64::from_str_radix(removed_0x.trim(), 16);
                    let result = match base16_num {
                        Ok(num) => T::from_str(num.to_string().as_str()),
                        Err(e) => return Err(serde::de::Error::custom(e))
                    };
                    result.map(Some).map_err(serde::de::Error::custom)
                } else {
                    T::from_str(s.trim()).map(Some).map_err(serde::de::Error::custom)
                }
            }
        },
        NumericOrNull::FromStr(i) => Ok(Some(i)),
        NumericOrNull::Null => Ok(None),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SPUSBDataType {
    #[serde(rename(deserialize = "SPUSBDataType"))]
    buses: Vec<USBBus>
}

impl fmt::Display for SPUSBDataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for v in &self.buses {
            if f.alternate() {
                if f.sign_plus() {
                    writeln!(f, "{:+#}", v)?;
                } else {
                    writeln!(f, "{:#}", v)?;
                }
            } else if f.sign_plus() {
                write!(f, "{:+}", v)?;
            } else {
                write!(f, "{:}", v)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct USBBus {
    #[serde(rename(deserialize = "_name"))]
    name: String,
    host_controller: String,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pci_device: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pci_revision: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pci_vendor: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    usb_bus_number: Option<u8>,
    // devices are normally hubs
    #[serde(rename(deserialize = "_items"))]
    devices: Option<Vec<USBDevice>>
}

pub fn write_devices_recursive(f: &mut fmt::Formatter, devices: &Vec<USBDevice>) -> fmt::Result {
    for device in devices {
        // print the device details
        if f.alternate() {
            if f.sign_plus() {
                writeln!(f, "{:+#}", device)?;
            } else {
                writeln!(f, "{:#}", device)?;
            }
        } else if f.sign_plus() {
            writeln!(f, "{:+}", device)?;
        } else {
            writeln!(f, "{:}", device)?;
        }
        // print all devices with this device - if hub for example
        device.devices.as_ref().map(|d| write_devices_recursive(f, d));
    }
    Ok(())
}

impl fmt::Display for USBBus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // use plus formatter to add tree
        let tree: &str = if !f.sign_plus() {
            ""
        } else {
            if f.alternate() {
                if self.devices.is_some() {
                    "╓ "
                } else {
                    "- "
                }
            // lsusb tree
            } else {
                "/: "
            }
        };

        // write the bus details - alternative for coloured and apple info style
        if f.alternate() {
            writeln!(f, "{:}{:} {:} PCI Device: {:}:{:} Revision: {:04x}",
                tree.bright_black().bold(),
                self.name.blue(),
                self.host_controller.green(),
                format!("{:04x}", self.pci_vendor.unwrap_or(0xffff)).yellow().bold(),
                format!("{:04x}", self.pci_device.unwrap_or(0xffff)).yellow(),
                self.pci_revision.unwrap_or(0xffff),
            )?;
        // lsusb style but not really accurate...
        } else { 
            writeln!(f, "{:}Bus {:03} Device 000 ID {:04x}:{:04x} {:} {:}",
                tree,
                self.usb_bus_number.unwrap_or(0),
                self.pci_vendor.unwrap_or(0xffff),
                self.pci_device.unwrap_or(0xffff),
                self.name,
                self.host_controller,
            )?;
        }
        // followed by devices if there are some
        self.devices.as_ref().map(|d| write_devices_recursive(f, d));
        Ok(())
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct USBDevice {
    #[serde(rename(deserialize = "_name"))]
    name: String,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    vendor_id: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    product_id: Option<u16>,
    location_id: String,
    serial_num: Option<String>,
    manufacturer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    bcd_device: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    bus_power: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    bus_power_used: Option<u16>,
    device_speed: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    extra_current_used: Option<u8>,
    // devices can be hub and have devices attached
    #[serde(rename(deserialize = "_items"))]
    devices: Option<Vec<USBDevice>>
}

impl fmt::Display for USBDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // # Build a formatted line to be sorted for the tree.
        // # The LocationID has the tree structure (0xbbdddddd):
        // #   0x  -- always
        // #   bb  -- bus number in hexadecimal
        // #   dddddd -- up to six levels for the tree, each digit represents its
        // #             position on that level
        // location_id is "location_reg / port"
        let location_split: Vec<&str> = self.location_id.split("/").collect();
        let reg = location_split.first().unwrap_or(&"0x00000000").trim().trim_start_matches("0x");
        let device_no = location_split.last().unwrap_or(&"0").trim().parse::<u32>().unwrap_or(1);
        // bus no is msb
        let bus_no = u32::from_str_radix(&reg, 16).unwrap_or(0) >> 24;
        // get position in tree based on number of non-zero chars or just 0 if not using tree
        let mut spaces = if f.sign_plus() {
            reg.get(2..).unwrap_or("0").trim_end_matches("0").len() * 4
        } else {
            0
        };

        // map speed from text back to data rate if tree
        let speed = match self.device_speed.as_ref().map(|s| s.as_str()) {
            Some("super_speed") => "5 Gb/s",
            Some("full_speed") => "12 Mb/s",
            Some("high_speed") => "480 Mb/s",
            Some(x) => x,
            None => "",
        };

        // tree chars to prepend if plus formatted
        let tree: &str = if !f.sign_plus() {
            ""
        } else {
            // TODO use "╟─ " unless last
            if f.alternate() {
                "╙── "
            } else {
                "|__ "
            }
        };

        // alternate for coloured, slightly different format to lsusb
        if f.alternate() {
            write!(f, "{:>spaces$}Bus {:} Device {:} ID {:}:{:} {:} {:} {:}", 
                   tree.bright_black(),
                   format!("{:03}", bus_no).cyan(),
                   format!("{:03}", device_no).magenta(),
                   format!("{:04x}", self.vendor_id.unwrap()).yellow().bold(), 
                   format!("{:04x}", self.product_id.unwrap()).yellow(), 
                   self.name.trim().blue(),
                   self.serial_num.as_ref().unwrap_or(&String::from("None")).trim().green(),
                   speed.purple()
                  )
        // not same data as lsusb when tree (show port, class, driver etc.)
        } else {
            // add 3 because lsusb is like this
            if spaces > 0 {
                spaces += 3;
            }
            write!(f, "{:>spaces$}Bus {:03} Device {:03} ID {:04x}:{:04x} {:}", 
                   tree,
                   bus_no,
                   device_no,
                   self.vendor_id.unwrap_or(0xffff),
                   self.product_id.unwrap_or(0xffff),
                   self.name.trim(),
            )
        }
    }
}

pub fn get_spusb() -> Result<SPUSBDataType, io::Error> {
    let output = if cfg!(target_os = "macos") {
        Command::new("system_profiler")
                .args(["-json", "SPUSBDataType"])
                .output()?
    } else {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "system_profiler is only supported on macOS"))
    };

    serde_json::from_str(String::from_utf8(output.stdout).unwrap().as_str())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_device() {
        let device_json = "{
              \"_name\" : \"Arduino Zero\",
              \"bcd_device\" : \"1.00\",
              \"bus_power\" : \"500\",
              \"bus_power_used\" : \"500\",
              \"device_speed\" : \"full_speed\",
              \"extra_current_used\" : \"0\",
              \"location_id\" : \"0x02110000 / 3\",
              \"manufacturer\" : \"Arduino LLC\",
              \"product_id\" : \"0x804d\",
              \"serial_num\" : \"6DC00ADC5053574C342E3120FF122422\",
              \"vendor_id\" : \"0x2341\"
            }";

        let device: USBDevice =
            serde_json::from_str(device_json).unwrap();

        assert_eq!(device.name, "Arduino Zero");
        assert_eq!(device.bcd_device, Some(1.00));
        assert_eq!(device.bus_power, Some(500));
        assert_eq!(device.bus_power_used, Some(500));
        assert_eq!(device.device_speed, Some("full_speed".to_string()));
        assert_eq!(device.extra_current_used, Some(0));
        assert_eq!(device.location_id, "0x02110000 / 3".to_string());
        assert_eq!(device.manufacturer, Some("Arduino LLC".to_string()));
        assert_eq!(device.product_id, Some(0x804d));
        assert_eq!(device.vendor_id, Some(0x2341));
    }

    #[test]
    fn test_deserialize_bus() {
        let device_json = "{
            \"_name\" : \"USB31Bus\",
            \"host_controller\" : \"AppleUSBXHCITR\",
            \"pci_device\" : \"0x15f0 \",
            \"pci_revision\" : \"0x0006 \",
            \"pci_vendor\" : \"0x8086 \",
            \"usb_bus_number\" : \"0x00 \"
        }";

        let device: USBBus =
            serde_json::from_str(device_json).unwrap();

        assert_eq!(device.name, "USB31Bus");
        assert_eq!(device.host_controller, "AppleUSBXHCITR");
        assert_eq!(device.pci_device, Some(0x15f0));
        assert_eq!(device.pci_revision, Some(0x0006));
        assert_eq!(device.pci_vendor, Some(0x8086));
        assert_eq!(device.usb_bus_number, Some(0x00));
    }
}
