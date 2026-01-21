```bash
           o
      o   /---o
     /---/---o
o---/
     \---\---o
      o   \---o
            o
```
# Cyme

[![Crates.io](https://img.shields.io/crates/v/cyme?style=flat-square)](https://crates.io/crates/cyme)
[![docs.rs](https://img.shields.io/docsrs/cyme?style=flat-square)](https://docs.rs/cyme/latest/cyme/)

List system USB buses and devices. A modern cross-platform `lsusb` that attempts to maintain compatibility with, but also add new features. Profiles system USB buses and the devices on those buses, including full device descriptors.

As a developer of embedded devices, I use a USB list tool on a frequent basis and developed this to cater to what I believe are the short comings of `lsusb`: verbose dump is mostly _too_ verbose, tree doesn't contain useful data on the whole, it barely works on non-Linux platforms and modern terminals support features that make glancing through the data easier.

The project started as a quick replacement for the barely working [lsusb script](https://github.com/jlhonora/lsusb) and a Rust project to keep me up to date! Like most fun projects, it quickly experienced feature creep as I developed it into a cross-platform replacement for `lsusb`. It started as a macOS `system_profiler` parser, evolved to include a 'libusb' based profiler for reading full device descriptors and now defaults to a pure Rust profiler using [nusb](https://github.com/kevinmehall/nusb).

It's not perfect as it started out as a Rust refresher but I had a lot of fun developing it and hope others will find it useful and can contribute. Reading around the [lsusb source code](https://github.com/gregkh/usbutils/blob/master/lsusb.c), USB-IF and general USB information was also a good knowledge builder.

The name comes from the technical term for the type of blossom on a Apple tree: [cyme](https://en.wikipedia.org/wiki/Inflorescence#Determinate_or_cymose) - it is Apple related and also looks like a USB device tree 😃🌸.

![cli tree output](./doc/cli-tree.png)

# Features

* Compatible with `lsusb` using `--lsusb` argument. Supports all arguments including `--verbose` output - fully parsed device descriptors! Output is identical for use with no args (list), tree (excluding drivers on non-Linux) and should match for verbose (perhaps formatting differences).
* Default build is a native Rust profiler using [nusb](https://docs.rs/nusb/latest/nusb).
* Filters like `lsusb` but that also work when printing `--tree`. Adds `--filter-name`, `--filter-serial`, `--filter-class` and option to hide empty `--hide-buses`/`--hide-hubs`.
* Improved `--tree` mode; shows device, configurations, interfaces and endpoints as tree depending on level of `--verbose`.
* Controllable display `--blocks` for device, bus `--bus-blocks`, configurations `--config-blocks`, interfaces `--interface-blocks` and endpoints `--endpoint-blocks`. Use `--more` to see more by default.
* Modern terminal features with coloured output, utf-8 characters and icon look-up based device data. Can be turned off and customised. See `--encoding` (glyphs [default], utf8 and ascii), which can keep icons/tree within a certain encoding, `--color` (auto [default], always and never) and `--icon` (auto [default], always and never). Auto `--icon` will only show icons if all icons to be shown are supported by the `--encoding`.
* Can be used as a library too with system profiler module, USB descriptor modules and `display` module for printing amongst others.
* `--json` output that honours filters and `--tree`.
* `--headers` to show meta data only when asked and not take space otherwise.
* `--mask-serials` to either '\*' or randomise serial string for sharing dumps with sensitive serial numbers.
* Auto-scaling to terminal width. Variable length strings such as descriptors will be truncated with a '...' to indicate this. Can be disabled with config option 'no-auto-width' and a fixed max defined with 'max-variable-string-len'.
* `cyme watch` subcommand to watch for USB device hotplug events and also live edit display settings. Works with all global flags.
* Targets for Linux, macOS and Windows.

## Demo

<a href="https://asciinema.org/a/542486" target="_blank"><img src="https://asciinema.org/a/542486.svg" /></a>

* [YouTube demo of watch sub-command](https://youtu.be/ohRBrVBRolA)

# Install

## Requirements

For pre-compiled binaries, see the [releases](https://github.com/tuna-f1sh/cyme/releases). Pre-compiled builds use native profiling backends and should require no extra dependencies.

From crates.io with a Rust tool-chain installed: `cargo install cyme`. To do it from within a local clone: `cargo install --locked --path .`.

### Package Managers

* [Homebrew 'cyme'](https://formulae.brew.sh/formula/cyme) which will also install a man page, completions and the 'libusb' dependency:

```bash
brew install cyme
```

* [Arch Linux official package](https://archlinux.org/packages/extra/x86_64/cyme/)

```bash
pacman -S cyme
```

* [Debian packages as part of release](https://github.com/tuna-f1sh/cyme/releases) - need a Debian maintainer for this.

More package managers to come/package distribution, please feel free to create a PR if you want to help out here.

## Alias `lsusb`

If one wishes to create a macOS version of lsusb or just use this instead, create an alias one's environment with the `--lsusb` compatibility flag:

`alias lsusb='cyme --lsusb'`

## Linux udev Information

> [!NOTE]
> Only supported on Linux targets.

To obtain device and interface drivers being used on Linux like `lsusb`, one can use the `--features udev` feature when building - it's a default feature. The feature uses the Rust crate [udevrs](https://crates.io/crates/udevrs) to obtain the information. To use the C FFI libudev library, use `--no-default-features --features udevlib` which will use the 'libudev' crate. Note that this will require 'libudev-dev' to be installed on the host machine.

To lookup USB IDs from the udev hwdb as well (like `lsusb`) use `--features udev_hwdb`. Without hwdb, `cyme` will use the 'usb-ids' crate, which is the same source as the hwdb binary data but the bundled hwdb may differ due to customisations or last update ('usb-ids' will be most up to date).

## Profilers and Feature Flags

### Native

Uses native Rust [nusb](https://docs.rs/nusb/latest/nusb) and [udevrs](https://crates.io/crates/udevrs) for profiling devices: sysfs (Linux), IOKit (macOS) and WinUSB.

It is the default profiler as of 2.0.0. Use `--feature=native` ('nusb' and 'udevrs' on Linux) or `--feature=nusb` to manually specify.

### Libusb

Uses 'libusb' for profiling devices. Requires [libusb 1.0.0](https://libusb.info) to be installed: `brew install libusb`, `sudo apt install libusb-1.0-0-dev` or one's package manager of choice.

Was the default feature before 2.0.0 for gathering verbose information. It is the profiler used by `lsusb` but there should be no difference in output between the two, since cyme uses control messages to gather the same information. If one wishes to use 'libusb', use `--no-default-features` and `--feature=libusb` or `--feature=ffi` for udevlib too.

> [!NOTE]
> 'libusb' does not profile buses on non-Linux systems (since it relies on root\_hubs). On these platforms, `cyme` will generate generic bus information.

### macOS `system_profiler`

Uses the macOS `system_profiler SPUSBDataType` command to profile devices.

Was the default feature before 2.0.0 for macOS systems to provide the base information; 'libusb' was used to open devices for verbose information. It is not used anymore if using the default native profiler but can be forced with `--system-profiler` - the native profiler uses the same IOKit backend but is much faster as it is not deserializing JSON. It also always captures bus numbers where `system_profiler` does not.

> [!TIP]
> If wishing to use only macOS `system_profiler` and not obtain more verbose information, remove default features with `cargo install --no-default-features cyme`. There is not much to be gained by this considering that the default native profiler uses the same IOKit as a backend, can open devices to read descriptors (verbose mode) and is much faster.

# Usage

Use `cyme --help` for basic usage or `man ./doc/cyme.1`. There are also autocompletions in './doc'.

## Examples

### Tree

```bash
# List all USB devices and buses in a tree format with default display blocks
cyme --tree
# As above but with configurations too
cyme --tree --verbose
# And with interfaces and endpoints - each verbose level goes further down the USB descriptor tree. Using short arg here.
cyme --tree -vvv
# List all USB devices and buses in a tree format with more display blocks, all verbose levels and headings to show what is being displayed
cyme --tree --more --headings
# Export the tree to a JSON file - --json works with all options
cyme --tree --verbose --json > tree.json
# Then import the JSON file to view the system USB tree as it was when exported. All cyme args can be used with this static import as if it was profiled data.
cyme --from-json tree.json
```

### lsusb

```bash
# List all USB devices and buses like 'lsusb'
cyme --lsusb
# lsusb verbose device dump including all descriptor information
cyme --lsusb --verbose
# lsusb tree mode (can add verbose levels [-v])
cyme --lsusb --tree
```

### Blocks

See `cyme --help` for blocks available. One can also omit the value to the arg to show options. Specifying multiple blocks requires multiple args or csv. By default the supplied blocks will replace the default blocks. Use `--block-operation` to change this behaviour.

```bash
# List USB devices with more display blocks
cyme --more
# List USB devices with chosen blocks: name, vid, pid, serial, speed (can use short -b)
cyme --blocks name,vendor-id,product-id,serial -b speed
# Customise other blocks - it's probably easier to use Config at this point. -vvv to see config, interfaces and endpoints
cyme -vvv --blocks name --bus-blocks name --config-blocks name --interface-blocks class --endpoint-blocks number
# Use block-operation to change the default or config blocks with arg blocks
cyme --block-operation remove --blocks serial
cyme --block-operation add --blocks base-class --blocks last-event --bus-blocks host-controller-vendor
```

### Filtering

```bash
# Filter for only Apple devices (vid:pid is base16)
cyme -d 0x05ac
# Specifically an Apple Headset, masking the serial number with '*'
cyme -d 05ac:8103 --mask-serials hide
# Filter for only devices with a certain name and class (filters can be combined)
cyme --filter-name "Black Magic" --filter-class cdc-data
```

### JSON - jq

```bash
# Find /dev/tty devices for named CDC ACM device (replace with --filter-name with -d vid:pid for more specific filtering)
cyme --filter-class cdc-communications --filter-name 'esp' --json | jq '.[] | (.extra.configurations[].interfaces[].devpaths[0]) | select(. != null)'
# Find mount points for mass storage devices
cyme --filter-class mass-storage --json | jq '.[] | {device_name: .name, devpaths: .extra.configurations[].interfaces[].devpaths, mounts: .extra.configurations[].interfaces[].mount_paths}'
# Dump newly connected devices only (using cyme watch)
cyme --json watch | jq '.buses[] | .devices[]? | select( (.last_event | has("connected")))'
```

#### Functions

```bash
ttyvid() {
  cyme --json --filter-class cdc-communications -d $1 | jq '.[] | (.extra.configurations[].interfaces[].devpaths[0]) | select(. != null)' | tr -d '"'
}

ttyname() {
  cyme --json --filter-class cdc-communications --filter-name $1 | jq '.[] | (.extra.configurations[].interfaces[].devpaths[0]) | select(. != null)' | tr -d '"'
}
```

Then use with a serial IO tool to open a device based on VID or name: `tio $(ttyname 'edbg')`

## Crate

For usage as a library for profiling system USB devices, the crate is 100% documented so look at [docs.rs](https://docs.rs/cyme/latest/cyme/). The main useful modules for import are [profiler](https://docs.rs/cyme/latest/cyme/profiler/index.html), and [usb](https://docs.rs/cyme/latest/cyme/usb/index.html).

There are also some examples in 'examples/', these can be run with `cargo run --example filter_devices`. It wasn't really written from the ground-up to be a crate but all the USB descriptors might be useful for high level USB profiling.

## Config

`cyme` will check for a 'cyme.json' config file in:

* Linux: "$XDG\_CONFIG\_HOME/cyme or $HOME/.config/cyme"
* macOS: "$HOME/Library/Application Support/cyme"
* Windows: "{FOLDERID\_RoamingAppData}/cyme"

One can also be supplied with `--config`. Copy or refer to './doc/cyme\_example\_config.json' for configurables. The file is essentially the default args; supplied args will override these. Use `--debug` to see where it is looking or if it's not loading.

`cyme watch` can also be used to live edit display settings then save the config to the default location with 'Ctrl-s'. It's probably the easiest way to customise display blocks.

### Custom Icons and Colours

See './doc/cyme\_example\_config.json' for an example of how icons can be defined and also the [docs](https://docs.rs/cyme/latest/cyme/icon/enum.Icon.html). The config can exclude the "user"/"colours" keys if one wishes not to define any new icons/colours.

Icons are looked up in an order of User -> Default. For devices: `Name` -> `VidPid` -> `VidPidMsb` -> `Vid` -> `UnknownVendor` -> `get_default_vidpid_icon`, classes: `ClassifierSubProtocol` -> `Classifier` -> `UndefinedClassifier` -> `get_default_classifier_icon`. User supplied colours override all internal; if a key is missing, it will be `None`.

#### Icons not Showing/Boxes with Question Marks

Copied from [lsd](https://github.com/lsd-rs/lsd#icons-not-showing-up): For `cyme` to be able to display icons, the font has to include special font glyphs. This might not be the case for most fonts that you download. Thankfully, you can patch most fonts using [NerdFont](https://www.nerdfonts.com/) and add these icons. Or you can just download an already patched version of your favourite font from [NerdFont font download page](https://www.nerdfonts.com/font-downloads).
Here is a guide on how to setup fonts on [macOS](https://github.com/Peltoche/lsd/issues/199#issuecomment-494218334) and [Android](https://github.com/Peltoche/lsd/issues/423).

To check if the font you are using is setup correctly, try running the following snippet in a shell and see if that [prints a folder icon](https://github.com/Peltoche/lsd/issues/510#issuecomment-860000306). If it prints a box, or question mark or something else, then you might have some issues in how you setup the font or how your terminal emulator renders the font.

```sh
echo $'\uf115'
```

If one does not want icons, provide a config file with custom blocks not including the any 'icon\*' blocks - see the example config. Alternatively, to only use standard UTF-8 characters supported by all fonts (no private use area) pass `--encoding utf8` and `--icon auto` (default). The `--icon auto` will drop the icon blocks if the characters matched are not supported by the `--encoding`.

For no icons at all, use the hidden `--no-icons` or `--icon never` args.

# Known Issues

* `sudo` is required to open and read Linux root\_hub string descriptors and potentially all devices if the user does not have [permissions](https://docs.rs/nusb/latest/nusb/#linux). The program works fine without these however, as will use sysfs/hwdb/'usb-ids' like lsusb. Use debugging `-z` to see what devices failed to read. The env CYME_PRINT_NON_CRITICAL_PROFILER_STDERR can be used to print these to stderr. `--lsusb --verbose` will print a message to stderr always to match the 'lsusb' behaviour.
* Users cannot open special non-user devices on Apple buses (VHCI); T2 chip for example. These will still be listed with 'native' and `system_profiler` but not `--force-libusb`. They will not print verbose information however and log an error if `--verbose` is used/print if `--lsusb`.
