# Changelog

## Unreleased

### Fixed

- lsusb verbose would print all audio BmControl2 bits and show ILLEGAL VALUE for 0 bits.

## [2.1.0] - 2024-10-30

### Fixed

- Linux root\_hub missing from it's own devices; lsusb mode listing with libusb feature.
- nusb feature not profiling root\_hub devices and so not gathering BOS, hub descriptors and status etc.
- Attempt to claim HID interfaces on Linux to avoid dmesg warning. Note that if a kernel module is loaded for a device, it may still be claimed by that module and so not available to cyme. cyme could detach the kernel module but this is not done for usability reasons. The behaviour is the same as lsusb. ([#52](https://github.com/tuna-f1sh/cyme/pull/52)).

### Changed

- Logging should be more useful in debug mode.

## [2.0.0] - 2024-10-18

Big release after almost two years since the first commit: `cyme` is now native Rust\* by default! Thanks to support from [nusb](https://github.com/kevinmehall/nusb), the system profiler is much improved for all platforms.

See the updated README for target configuration changes.

\*Native crates. The OS interfaces behind the scenes (currently sysfs, IOKit and WinUSB) are in their respective code but this opens the door for Rust OSes, which the previous 'libusb' profiler could not facilitate.

### Added

- Bus information is now profiled on non-Linux platforms using 'nusb' - much nicer output for macOS and Windows.
- pci.ids vendor and device information for buses where IDs are available.

### Changed

- `cyme` default target now uses native Rust profiling thanks to [nusb](https://github.com/kevinmehall/nusb) ([#26](https://github.com/tuna-f1sh/cyme/pull/26)).
- Default Driver and Interface display blocks now include driver and sysfs on Linux but not on other platforms ([#41](https://github.com/tuna-f1sh/cyme/issues/41)).
- macOS `system_profiler` is not used by default with 'nusb' since IOKit is used directly. It can be forced with `--system_profiler`. The macOS mod is now only compiled for macOS targets.
- 'sysfs' read/readlink is now attempted first for Linux driver information then udev (if feature enabled) ([#45](https://github.com/tuna-f1sh/cyme/pull/45)).

## [1.8.5] - 2024-10-11

### Added

- risv64 support ([#37](https://github.com/tuna-f1sh/cyme/pull/37)).

### Fixed

- MixerUnit1 number of channels index incorrect causing OoB panic ([#38](https://github.com/tuna-f1sh/cyme/issues/38)).

## [1.8.4] - 2024-09-27

### Changed

- Default sort by bus number and device address within buses for all display modes (matching lsusb) ([#33](https://github.com/tuna-f1sh/cyme/issues/33)).
- Default Rust udev feature no longer supports hwdb lookup as it's broken - usb-ids is used. Use `--no-default-features -F=udevlib -F=udev_hwdb` if really wishing to use local 'hwdb.bin'. ([#35](https://github.com/tuna-f1sh/cyme/issues/35)).

## [1.8.3] - 2024-09-20

### Fixes

- Fix panic when using auto-width and utf-8 characters landing on non-char boundary ([#30](https://github.com/tuna-f1sh/cyme/issues/32)).
- Corrected some typos ([#28](https://github.com/tuna-f1sh/cyme/pull/28)).
- Fix lintian errors with cargo-deb package ([#29](https://github.com/tuna-f1sh/cyme/pull/31)).

## [1.8.2] - 2024-08-20

### Changed

- Standard cyme list now excludes root_hubs (`--tree` shows them as buses as before). `--lsusb` list mode will still show them. Use `--list-root-hubs` (or in config) to include them in the cyme list on Linux as before.

### Fixes

- Fix length and offset calculation in lsusb::dump_hub that would print some incorrect data.
- Minor formatting fixes for `lsusb --verbose` mode; indent in dump_interface, min 1 space between fields, wTotalLength as hex.

## [1.8.1] - 2024-07-16

### Fixes

- Fix panic due to potential subtraction overflow in `lsusb --verbose` mode ([#24](https://github.com/tuna-f1sh/cyme/issues/25)).

## [1.8.0] - 2024-07-15

`cyme` should now match `lsusb --verbose` mode with full device descriptor dumps, including using USB control messages to get BOS, Hub device status, HID reports and more. It's been a lot of grunt work and lines of code (not very creative lines!) creating all the types but it should be useful as a USB profiling crate moving forwards and I think more robust than `lsusb` in some cases. There may still be some formatting differences but the data _should_ be the same. `cyme` without `--lsusb --verbose` display isn't changed for the most part, since the dumping is extremely device specific and verbose. I may add device status as a display block in future.

### Added

- Full dumps of device descriptors for matching `--lsusb --verbose` [#23](https://github.com/tuna-f1sh/cyme/pull/23) ([#15](https://github.com/tuna-f1sh/cyme/issues/15))
- Device name pattern matching for icon with `Icon::name(String)` ([#22](https://github.com/tuna-f1sh/cyme/pull/22))

### Changed

- `cyme` is now in [Homebrew core](https://formulae.brew.sh/formula/cyme). One can `brew uninstall cyme`, `brew untap tuna-f1sh/taps`, then install with `brew install cyme` ([#21](https://github.com/tuna-f1sh/cyme/pull/21)).
- Update `--lsusb` mode to match updated lsusb behaviour if driver/names missing (print '[none]'/'[unknown]').

## [1.7.0] - 2024-06-25

### Changed

- Replace [udev-rs](https://github.com/Smithay/udev-rs) and indirectly libudev-sys with Rust native [udev](https://github.com/cr8t/udev); libudev dependency (and system requirement) is now optional but can be used with `--no-default-features -F=udevlib`. ([#19](https://github.com/tuna-f1sh/cyme/pull/19))

### Fixes

- Replace more font-awesome icons in default look-up that have been deprecated ([#20](https://github.com/tuna-f1sh/cyme/issues/20))

## [1.6.1] - 2024-06-13

### Fixes

- Replace font-awesome icons in default look-up that have been deprecated.

## [1.6.0] - 2023-11-23

_A release of patches, PRs and merges :), thanks to support_

### Added

- Support udev/sysfs iString lookup ([#14](https://github.com/tuna-f1sh/cyme/pull/14)) (@haata).
- Add fully defined USB Device based on class code triplet.
- Support bLength, wTotalLength and bDescriptorType fields in `lsusb --verbose` with ([rusb/#185](https://github.com/a1ien/rusb/pull/185)). This completes the `lsusb --verbose` support apart from extra descriptors.
- Add `lsusb::names` mod that ports 'usbutils/names.c' to match the behaviour using `lsusb --verbose`. This means class, sub-class and protocol udev-hwdb names are included now in `lsusb --verbose` ([b99e87](https://github.com/tuna-f1sh/cyme/commit/b99e87a586248fdd6dbf72d5624e5e61e993ff5a)). 
- Add the display blocks `uid-class`, `uid-subc-lass`, `uid-protocol`, `class`, and `class-value` for `DeviceBlock`s and `InterfaceBlock`s. These are also added for `--more`.
- Add `feature=udev_hwdb` to guard against systems that have udev but not hwdb support ([cross/#1377](https://github.com/cross-rs/cross/issues/1377))/([libudev-sys/#16](https://github.com/dcuddeback/libudev-sys/pull/16)).

### Changed

- 'usb-ids' crate is now a dependency rather than optional to support `lsusb::names` lookup without udev_hwdb (non-Linux). ([usb-ids.rs/#50](https://github.com/woodruffw/usb-ids.rs/pull/50)) will add extra descriptor parsing in future.
- iString descriptors will now be retrieved in order libusb descriptor -> sysfs cache (Linux) -> udev_hwdb (bundled usb-ids `--feature=udev_hwdb`) -> usb-ids.

### Fixes

- Fix BaseClass as u8 in lsusb --verbose being enum index not repr(c) base class byte.
- Fix BaseClass as u8 in icon serializer being enum index not repc(c) base class byte.

## [1.5.2] - 2023-11-01

_Changelog started._

## [0.2.0] - 2022-11-16

_First release._
