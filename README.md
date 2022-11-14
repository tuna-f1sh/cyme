# Cyme

![cli tree output](./img/cli-tree.png)

A macOS `system_profiler SPUSBDataType` parser and utility that attempts to maintain compatibility with but also build upon `lsusb`. The project started as a quick replacement for the barely working [lsusb script](https://github.com/jlhonora/lsusb) and is my _yearly_ Rust project to keep up to date!

I'd like to build this into a modern `lsusb` tool, akin to `lsd`, `fd` etc. It is currently in flux as I build the foundations, learn about custom serde Deserializers and newer Rust features.

The name comes from the technical term for the type of blossom on a Apple tree: [cyme](https://en.wikipedia.org/wiki/Inflorescence#Determinate_or_cymose) - it is Apple related and also looks like a USB device tree 😃🌸.

## Planned Features

* Controllable block data like `lsd --blocks`
* Vendor ID nerd font icon look up and icon theming like `lsd --icon`.
* Interface and Device Descriptor icon look up.
* Modern drawing of device tree with utf-8 boxes.
* Tree support of all device interfaces and endpoints.

# Install

Clone this directory and with a Rust toolchain installed: `cargo install --path cyme`

## Alias `lsusb`

If you want to create a macOS version of lsusb, create an alias in your environment with the `--lsusb` compatibility flag:

`alias lsusb='cyme --lsusb'`

# Usage

Will cover this more as it develops. Use `cyme --help` for basic usage.
