use std::fmt;
use std::str::FromStr;

use std::process::Command;
use serde::{Deserialize, Deserializer, Serialize};

/// borrowed from https://github.com/vityafx/serde-aux/blob/master/src/field_attributes.rs with addition of base16 encoding
/// Deserializes an option number from string or a number.
pub fn deserialize_option_number_from_string<'de, T, D>(
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
                let removed_0x = s.trim_start_matches("0x");
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
struct SPUSBDataType {
    #[serde(rename(deserialize = "SPUSBDataType"))]
    buses: Vec<USBBus>
}

#[derive(Debug, Serialize, Deserialize)]
struct USBBus {
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
    usb_bus_number: Option<u16>,
    // devices are normally hubs
    #[serde(rename(deserialize = "_items"))]
    devices: Option<Vec<USBDevice>>
}

#[derive(Debug, Serialize, Deserialize)]
struct USBDevice {
    #[serde(rename(deserialize = "_name"))]
    name: String,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    vendor_id: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    product_id: Option<u16>,
    location_id: String,
    serial_num: String,
    manufacturer: String,
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

fn get_system_profile() -> SPUSBDataType {
    let output = if cfg!(target_os = "macos") {
        Command::new("system_profiler")
                .args(["-json", "SPUSBDataType"])
                .output()
                .expect("failed to execute process")
    } else {
        Command::new("lsusb")
                .arg("-c")
                .output()
                .expect("failed to execute process")
    };

    // return output.stdout;

    let json_args: SPUSBDataType =
        serde_json::from_str(String::from_utf8(output.stdout).unwrap().as_str()).expect(&format!("Failed to parse output"));

    return json_args;
}

fn main() {
    println!("{:#?}", get_system_profile());
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
        println!("{:#?}", device);
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
        println!("{:#?}", device);
    }
}
