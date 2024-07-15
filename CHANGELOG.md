# Changelog

## [1.8.0] - 2024-07-14

`cyme` should now match `lsusb --verbose` mode with full device descriptor dumps, including using USB control messages to get BOS, Hub device status, HID reports and more. It's been a lot of grunt work and lines of code (not very creative lines!) creating all the types but it should be useful as a USB profiling crate moving forwards and I think more robust than `lsusb` in some cases. There may still be some formatting differences but the data _should_ be the same. `cyme` without `--lsusb --verbose` display isn't changed for the most part, since the dumping is extremely device specific and verbose. I may add device status as a display block in future.

### Addded

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

- Fix ClassCode as u8 in lsusb --verbose being enum index not repr(c) base class byte.
- Fix ClassCode as u8 in icon serializer being enum index not repc(c) base class byte.

## [1.5.2] - 2023-11-01

_Changelog started._

## [0.2.0] - 2022-11-16

_First release._
