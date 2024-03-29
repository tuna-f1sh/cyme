# Changelog

## [Unreleased]

- Working on full dumps of device descriptors ([#15](https://github.com/tuna-f1sh/cyme/issues/15))

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
