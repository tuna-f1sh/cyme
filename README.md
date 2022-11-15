```
           o
      o   /---o
     /---/---o
o---/
     \---\---o
      o   \---o
            o
```
# Cyme

A macOS `system_profiler SPUSBDataType` parser and utility that attempts to maintain compatibility with but also build upon `lsusb`. The project started as a quick replacement for the barely working [lsusb script](https://github.com/jlhonora/lsusb) and is my _yearly_ Rust project to keep up to date!

I'd like to build this into a modern `lsusb` tool, akin to `lsd`, `fd` etc. It is currently in flux as I build the foundations, learn about custom serde Deserializers and newer Rust features.

The name comes from the technical term for the type of blossom on a Apple tree: [cyme](https://en.wikipedia.org/wiki/Inflorescence#Determinate_or_cymose) - it is Apple related and also looks like a USB device tree 😃🌸.

![cli tree output](./img/cli-tree.png)

# Features

* Compatible with `lsusb` using `--lsusb` argument. Supports all arguments including `--verbose` output using [libusb](https://github.com/dcuddeback/libusb-rs).
* Filters like `lsusb` but that also work when printing `--tree`. Adds `--filter_name`, `filter_serial` and option to hide empty `--hide-buses`/`--hide-hubs`.
* Modern terminal features with coloured output, utf-8 characters and icons.
* Works as a library too with `system_profiler` parsing crate and `lsusb` crate for libusb.

## Planned Features

* Controllable block data like `lsd --blocks`
* Vendor ID nerd font icon look up and icon theming like `lsd --icon`.
* Modern drawing of device tree with utf-8 boxes.
* Tree support of all device interfaces and endpoints.
* libusb optional to get more USB data and support other OS with switch from `system_profiler`.
* Interface and Device Descriptor icon look up.

# Install

From crates.io with a Rust tool-chain installed: `cargo install cyme`. If wishing to do it from within a local clone: `cargo install --path .`.

If wishing to use full `lsusb` support, include the 'libusb' feature with `cargo install --features libusb cyme`

I also have a Homebrew tap: `brew tap tuna-f1sh/cyme`.

## Alias `lsusb`

If you want to create a macOS version of lsusb, create an alias in your environment with the `--lsusb` compatibility flag:

`alias lsusb='cyme --lsusb'`

The `--verbose` argument requires the 'libusb' feature - see Install.

Examples output:

```
> lsusb
Bus 000 Device 001: ID 0bda:0411 4-Port USB 3.0 Hub
Bus 000 Device 002: ID 0bda:0411 4-Port USB 3.0 Hub
Bus 002 Device 002: ID 043e:9a60 USB3.1 Hub
Bus 002 Device 004: ID 2109:0817 USB3.0 Hub
Bus 002 Device 007: ID 0781:558c Extreme SSD
Bus 002 Device 008: ID 0bda:8153 Belkin USB-C LAN
Bus 002 Device 006: ID 043e:9a71 hub_device
Bus 002 Device 009: ID 043e:9a68 LG UltraFine Display Camera
Bus 002 Device 001: ID 043e:9a61 USB2.1 Hub
Bus 002 Device 005: ID 2109:2817 USB2.0 Hub
Bus 002 Device 012: ID 2109:8817 USB Billboard Device
Bus 002 Device 003: ID 043e:9a73 hub_device
Bus 002 Device 011: ID 043e:9a70 LG UltraFine Display Controls
Bus 002 Device 010: ID 043e:9a66 LG UltraFine Display Audio
Bus 020 Device 001: ID 0bda:5411 4-Port USB 2.0 Hub
Bus 020 Device 002: ID 0bda:5411 4-Port USB 2.0 Hub
```

# Usage

Will cover this more as it develops. Use `cyme --help` for basic usage.
