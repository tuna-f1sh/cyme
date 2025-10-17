# Changelog

## [2.2.7] - 2025-10-17

### Fixed

- config: some settings not being applied from config file ([#82](https://github.com/tuna-f1sh/cyme/pull/82)).
- args: sort args not being applied ([#82](https://github.com/tuna-f1sh/cyme/pull/82)).

## [2.2.6] - 2025-09-22

### Fixed

- watch: panic exiting due to potential dropped channel send ([#79](https://github.com/tuna-f1sh/cyme/pull/79)).

## [2.2.5] - 2025-09-18

### Fixed

- config: encoding arg not being passed to print settings ([#77](https://github.com/tuna-f1sh/cyme/pull/77)).

## [2.2.4] - 2025-07-28

### Changed

- Update to use release v0.2.0 of nusb.

## [2.2.3] - 2025-06-18

### Added

- `--block-operation` add to control how block args are processed: new, add, append, prepend or remove from config/defaults ([#72](https://github.com/tuna-f1sh/cyme/pull/72)).
- Block args can take comma separated values ([#72](https://github.com/tuna-f1sh/cyme/pull/72)).
- Default block args in `--help` ([#71](https://github.com/tuna-f1sh/cyme/issues/71)).

### Changed

- nusb v0.2.0-beta.2.

## [2.2.2] - 2025-05-20

### Fixed

- watch: bus assignment based on Windows bus_id ([#68](https://github.com/tuna-f1sh/cyme/pull/68)).

## [2.2.1] - 2025-05-06

### Added

- Support for `--json` with `watch` sub-command ([#66](https://github.com/tuna-f1sh/cyme/pull/66)).

## [2.2.0] - 2025-04-21

`cyme watch` subcommand to watch for USB device hotplug events and _live_ edit display settings. A simple TUI proof of concept that grew beyond just showing hotplug events into something quite handy for exploring enumerated devices. It can also edit display settings and save them to the cyme config file. 

It's a nice way to customise display blocks: press 'b' to use the editor and 'Ctrl-s' to save. Use '?' for other keybindings. Navigation is mostly Vim-like. Try connecting and disconnecting a device while running `cyme watch` to see the hotplug events.

The interface is simplistic at the moment but could be re-skinned with something like Ratatui in the future.

Here's a quick demo: https://youtu.be/ohRBrVBRolA?si=OY8zEtqF-8x_Lp7u

### Added

- `cyme watch` subcommand to watch for USB device hotplug events and 'live' edit display settings ([#58](https://github.com/tuna-f1sh/cyme/pull/58)).
- no_color option to config. Clearer parity/merge with CLI args.
- device event and DeviceBlock::LastEvent, DeviceBlock::EventIcon. Default is profiled time (P: %y-%m-%d %H:%M:%S) but used by `cyme watch` to show connect/disconnect events.
- benches for profiling.
- RUST_LOG can be module level eg: `RUST_LOG=nusb=info,cyme=debug`.

### Changed

- build: Makefile targets used in CI ([#64](https://github.com/tuna-f1sh/cyme/pull/64)).
- custom PortPath type used for get_ methods improves look-up 70-100%. Makes profiler faster as it uses these methods to build the device tree. ([801aa](https://github.com/tuna-f1sh/cyme/commit/801aa3fba28aae7be988d747b1a42bedbc06e496)).
- simple_logger now logs to stderr so can be redirected without effecting display output: `cyme watch 2> log`.
- path args String to PathBuf.

## [2.1.3] - 2024-04-03

### Fixed

- lsusb-verbose: hub dump not reading full descriptor for bcd >= 0x0300 so missing hub descriptor ([#63](https://github.com/tuna-f1sh/cyme/pull/63)).
- lsusb-verbose: verbose white space and some strings.

### Changed

- build: hide `--gen` behind `cli_generate` feature ([#61](https://github.com/tuna-f1sh/cyme/pull/61)).
- lsusb: brought upto date with v018 releae and some pre-v019 features ([#62](https://github.com/tuna-f1sh/cyme/pull/62)).

### Added

- display: negotiated-speed block to show the actual operating speed of the connected device.

## [2.1.2] - 2024-02-21

Mostly housekeeping and minor fixes. Did a dependency audit and updated some crates. Working towards a hotplug 'watch' subcommand.

### Fixed

- control read endpoint stall will be re-attempted after clearing halt ([#54](https://github.com/tuna-f1sh/cyme/pull/54)).
- udev-hwdb: native supports hwdb lookup again ([#59](https://github.com/tuna-f1sh/cyme/pull/59)).
- lsusb: fallback to desccriptor strings in verbose dump for idProduct and idVendor ([#55](https://github.com/tuna-f1sh/cyme/issues/55)).
- Bus::is_empty was inverse but display::prepare_devices filter accounted by also inverting. No real bug but fixed for clarity.

### Changed

- macOS: claim interface when reading Debug Descriptors.
- nusb: use cached device descriptor rather than reading manually with control message ([nusb #102](https://github.com/kevinmehall/nusb/pull/102)).
- log now outputs to stderr so can be redirected.
- lazy_static dropped for LazyLock.
- rand replaced with fast_rand.

### Added

- Example usage in README.

## [2.1.1] - 2024-12-01

Minor updates to match `lsusb` updates. Fixing bugs playing with USB gadgets!

### Fixed

- Linux root_hubs now read_link pci driver like lsusb for driver field.
- lsusb verbose would print all audio BmControl2 bits and show ILLEGAL VALUE for 0 bits.

### Changed

- lsusb tree number padding is now 3 digits for bus and device numbers to match lsusb.

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
