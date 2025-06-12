# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_cyme_global_optspecs
	string join \n l/lsusb t/tree d/vidpid= s/show= D/device= filter-name= filter-serial= filter-class= v/verbose b/blocks= bus-blocks= config-blocks= interface-blocks= endpoint-blocks= m/more sort-devices= sort-buses group-devices= hide-buses hide-hubs list-root-hubs decimal no-padding color= no-color encoding= ascii no-icons icon= headings json from-json= F/force-libusb c/config= z/debug mask-serials= gen system-profiler h/help V/version
end

function __fish_cyme_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_cyme_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_cyme_using_subcommand
	set -l cmd (__fish_cyme_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c cyme -n "__fish_cyme_needs_command" -s d -l vidpid -d 'Show only devices with the specified vendor and product ID numbers (in hexadecimal) in format VID:[PID]' -r
complete -c cyme -n "__fish_cyme_needs_command" -s s -l show -d 'Show only devices with specified device and/or bus numbers (in decimal) in format [[bus]:][devnum]' -r
complete -c cyme -n "__fish_cyme_needs_command" -s D -l device -d 'Selects which device lsusb will examine - supplied as Linux /dev/bus/usb/BBB/DDD style path' -r
complete -c cyme -n "__fish_cyme_needs_command" -l filter-name -d 'Filter on string contained in name' -r
complete -c cyme -n "__fish_cyme_needs_command" -l filter-serial -d 'Filter on string contained in serial' -r
complete -c cyme -n "__fish_cyme_needs_command" -l filter-class -d 'Filter on USB class code' -r -f -a "use-interface-descriptor\t'Device class is unspecified, interface descriptors are used to determine needed drivers'
audio\t'Speaker, microphone, sound card, MIDI'
cdc-communications\t'The modern serial interface; appears as a UART/RS232 port on most systems'
hid\t'Human Interface Device; game controllers, keyboards, mice etc. Also commonly used as a device data interface rather then creating something from scratch'
physical\t'Force feedback joystick'
image\t'Still imaging device; scanners, cameras'
printer\t'Laser printer, inkjet printer, CNC machine'
mass-storage\t'Mass storage devices (MSD): USB flash drive, memory card reader, digital audio player, digital camera, external drive'
hub\t'High speed USB hub'
cdc-data\t'Used together with class 02h (Communications and CDC Control) above'
smart-card\t'USB smart card reader'
content-security\t'Fingerprint reader'
video\t'Webcam'
personal-healthcare\t'Pulse monitor (watch)'
audio-video\t'Webcam, TV'
billboard\t'Describes USB-C alternate modes supported by device'
usb-type-c-bridge\t'An interface to expose and configure the USB Type-C capabilities of Connectors on USB Hubs or Alternate Mode Adapters'
bdp\t'This base class is defined for devices that conform to the “VESA USB BDP Device Specification” found at the VESA website. This specification defines the usable set of SubClass and Protocol values. Values outside of this defined spec are reserved. These class codes can only be used in Interface Descriptors'
mctp\t'This base class is defined for devices that conform to the “MCTP over USB” found at the DMTF website as DSP0283. This specification defines the usable set of SubClass and Protocol values. Values outside of this defined spec are reserved. These class codes can only be used in Interface Descriptors'
i3c-device\t'An interface to expose and configure I3C function within a USB device to allow interaction between host software and the I3C device, to drive transaction on the I3C bus to/from target devices'
diagnostic\t'Trace and debugging equipment'
wireless-controller\t'Wireless controllers: Bluetooth adaptors, Microsoft RNDIS'
miscellaneous\t'This base class is defined for miscellaneous device definitions. Some matching SubClass and Protocols are defined on the USB-IF website'
application-specific-interface\t'This base class is defined for devices that conform to several class specifications found on the USB-IF website'
vendor-specific-class\t'This base class is defined for vendors to use as they please'"
complete -c cyme -n "__fish_cyme_needs_command" -s b -l blocks -d 'Specify the blocks which will be displayed for each device and in what order. Supply arg multiple times to specify multiple blocks' -r -f -a "bus-number\t'Number of bus device is attached'
device-number\t'Bus issued device number'
branch-position\t'Position of device in parent branch'
port-path\t'Linux style port path'
sys-path\t'Linux udev reported syspath'
driver\t'Linux udev reported driver loaded for device'
icon\t'Icon based on VID/PID'
vendor-id\t'Unique vendor identifier - purchased from USB IF'
product-id\t'Vendor unique product identifier'
vid-pid\t'Unique vendor identifier and product identifier as a string formatted "vid:pid" like lsusb'
name\t'The device name as reported in descriptor or using usb_ids if None'
manufacturer\t'The device manufacturer as provided in descriptor or using usb_ids if None'
product-name\t'The device product name as reported by usb_ids vidpid lookup'
vendor-name\t'The device vendor name as reported by usb_ids vid lookup'
serial\t'Device serial string as reported by descriptor'
speed\t'Advertised device capable speed'
negotiated-speed\t'Negotiated device speed as connected'
tree-positions\t'Position along all branches back to trunk device'
bus-power\t'macOS system_profiler only - actually bus current in mA not power!'
bus-power-used\t'macOS system_profiler only - actually bus current used in mA not power!'
extra-current-used\t'macOS system_profiler only - actually bus current used in mA not power!'
bcd-device\t'The device version'
bcd-usb\t'The supported USB version'
base-class\t'Base class enum of interface provided by USB IF - only available when using libusb'
sub-class\t'Sub-class value of interface provided by USB IF - only available when using libusb'
protocol\t'Prototol value for interface provided by USB IF - only available when using libusb'
uid-class\t'Class name from USB IDs repository'
uid-sub-class\t'Sub-class name from USB IDs repository'
uid-protocol\t'Protocol name from USB IDs repository'
class\t'Fully defined USB Class Code enum based on BaseClass/SubClass/Protocol triplet'
base-value\t'Base class as number value rather than enum'
last-event\t'Last time device was seen'
event-icon\t'Event icon'"
complete -c cyme -n "__fish_cyme_needs_command" -l bus-blocks -d 'Specify the blocks which will be displayed for each bus and in what order. Supply arg multiple times to specify multiple blocks' -r -f -a "bus-number\t'System bus number identifier'
icon\t'Icon based on VID/PID'
name\t'System internal bus name based on Root Hub device name'
host-controller\t'System internal bus provider name'
host-controller-vendor\t'Vendor name of PCI Host Controller from pci.ids'
host-controller-device\t'Device name of PCI Host Controller from pci.ids'
pci-vendor\t'PCI vendor ID (VID)'
pci-device\t'PCI device ID (PID)'
pci-revision\t'PCI Revsision ID'
port-path\t'syspath style port path to bus, applicable to Linux only'"
complete -c cyme -n "__fish_cyme_needs_command" -l config-blocks -d 'Specify the blocks which will be displayed for each configuration and in what order. Supply arg multiple times to specify multiple blocks' -r -f -a "name\t'Name from string descriptor'
number\t'Number of config, bConfigurationValue; value to set to enable to configuration'
num-interfaces\t'Interfaces available for this configuruation'
attributes\t'Attributes of configuration, bmAttributes'
icon-attributes\t'Icon representation of bmAttributes'
max-power\t'Maximum current consumption in mA'"
complete -c cyme -n "__fish_cyme_needs_command" -l interface-blocks -d 'Specify the blocks which will be displayed for each interface and in what order. Supply arg multiple times to specify multiple blocks' -r -f -a "name\t'Name from string descriptor'
number\t'Interface number'
port-path\t'Interface port path, applicable to Linux'
base-class\t'Base class enum of interface provided by USB IF'
sub-class\t'Sub-class value of interface provided by USB IF'
protocol\t'Prototol value for interface provided by USB IF'
alt-setting\t'Interfaces can have the same number but an alternate settings defined here'
driver\t'Driver obtained from udev on Linux only'
sys-path\t'syspath obtained from udev on Linux only'
num-endpoints\t'An interface can have many endpoints'
icon\t'Icon based on BaseClass/SubCode/Protocol'
uid-class\t'Class name from USB IDs repository'
uid-sub-class\t'Sub-class name from USB IDs repository'
uid-protocol\t'Protocol name from USB IDs repository'
class\t'Fully defined USB Class Code based on BaseClass/SubClass/Protocol triplet'
base-value\t'Base class as number value rather than enum'"
complete -c cyme -n "__fish_cyme_needs_command" -l endpoint-blocks -d 'Specify the blocks which will be displayed for each endpoint and in what order. Supply arg multiple times to specify multiple blocks' -r -f -a "number\t'Endpoint number on interface'
direction\t'Direction of data into endpoint'
transfer-type\t'Type of data transfer endpoint accepts'
sync-type\t'Synchronisation type (Iso mode)'
usage-type\t'Usage type (Iso mode)'
max-packet-size\t'Maximum packet size in bytes endpoint can send/recieve'
interval\t'Interval for polling endpoint data transfers. Value in frame counts. Ignored for Bulk & Control Endpoints. Isochronous must equal 1 and field may range from 1 to 255 for interrupt endpoints'"
complete -c cyme -n "__fish_cyme_needs_command" -l sort-devices -d 'Sort devices operation' -r -f -a "device-number\t'Sort by bus device number'
branch-position\t'Sort by position in parent branch'
no-sort\t'No sorting; whatever order it was parsed'"
complete -c cyme -n "__fish_cyme_needs_command" -l group-devices -d 'Group devices by value when listing' -r -f -a "no-group\t'No grouping'
bus\t'Group into buses with bus info as heading - like a flat tree'"
complete -c cyme -n "__fish_cyme_needs_command" -l color -d 'Output coloring mode' -r -f -a "auto\t'Show colours if the output goes to an interactive console'
always\t'Always apply colouring to the output'
never\t'Never apply colouring to the output'"
complete -c cyme -n "__fish_cyme_needs_command" -l encoding -d 'Output character encoding' -r -f -a "glyphs\t'Use UTF-8 private use area characters such as those used by NerdFont to show glyph icons'
utf8\t'Use only standard UTF-8 characters for the output; no private use area glyph icons'
ascii\t'Use only ASCII characters for the output; 0x00 - 0x7F (127 chars)'"
complete -c cyme -n "__fish_cyme_needs_command" -l icon -d 'When to print icon blocks' -r -f -a "auto\t'Show icon blocks if the [`Encoding`] supports icons matched in the [`icon::IconTheme`]'
always\t'Always print icon blocks if included in configured blocks'
never\t'Never print icon blocks'"
complete -c cyme -n "__fish_cyme_needs_command" -l from-json -d 'Read from json output rather than profiling system' -r -F
complete -c cyme -n "__fish_cyme_needs_command" -s c -l config -d 'Path to user config file to use for custom icons, colours and default settings' -r -F
complete -c cyme -n "__fish_cyme_needs_command" -l mask-serials -d 'Mask serial numbers with \'*\' or random chars' -r -f -a "hide\t'Hide with \'*\' char'
scramble\t'Mask by randomising existing chars'
replace\t'Mask by replacing length with random chars'"
complete -c cyme -n "__fish_cyme_needs_command" -s l -l lsusb -d 'Attempt to maintain compatibility with lsusb output'
complete -c cyme -n "__fish_cyme_needs_command" -s t -l tree -d 'Dump USB device hierarchy as a tree'
complete -c cyme -n "__fish_cyme_needs_command" -s v -l verbose -d 'Verbosity level (repeat provides count): 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and more blocks'
complete -c cyme -n "__fish_cyme_needs_command" -s m -l more -d 'Print more blocks by default at each verbosity'
complete -c cyme -n "__fish_cyme_needs_command" -l sort-buses -d 'Sort devices by bus number. If using any sort-devices other than no-sort, this happens automatically'
complete -c cyme -n "__fish_cyme_needs_command" -l hide-buses -d 'Hide empty buses when printing tree; those with no devices'
complete -c cyme -n "__fish_cyme_needs_command" -l hide-hubs -d 'Hide empty hubs when printing tree; those with no devices. When listing will hide hubs regardless of whether empty of not'
complete -c cyme -n "__fish_cyme_needs_command" -l list-root-hubs -d 'Show root hubs when listing; Linux only'
complete -c cyme -n "__fish_cyme_needs_command" -l decimal -d 'Show base16 values as base10 decimal instead'
complete -c cyme -n "__fish_cyme_needs_command" -l no-padding -d 'Disable padding to align blocks - will cause --headings to become maligned'
complete -c cyme -n "__fish_cyme_needs_command" -l no-color -d 'Disable coloured output, can also use NO_COLOR environment variable'
complete -c cyme -n "__fish_cyme_needs_command" -l ascii -d 'Disables icons and utf-8 characters'
complete -c cyme -n "__fish_cyme_needs_command" -l no-icons -d 'Disables all Block icons by not using any IconTheme. Providing custom XxxxBlocks without any icons is a nicer way to do this'
complete -c cyme -n "__fish_cyme_needs_command" -l headings -d 'Show block headings'
complete -c cyme -n "__fish_cyme_needs_command" -l json -d 'Output as json format after sorting, filters and tree settings are applied; without -tree will be flattened dump of devices'
complete -c cyme -n "__fish_cyme_needs_command" -s F -l force-libusb -d 'Force pure libusb profiler on macOS rather than combining system_profiler output'
complete -c cyme -n "__fish_cyme_needs_command" -s z -l debug -d 'Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE'
complete -c cyme -n "__fish_cyme_needs_command" -l gen -d 'Generate cli completions and man page'
complete -c cyme -n "__fish_cyme_needs_command" -l system-profiler -d 'Use the system_profiler command on macOS to get USB data'
complete -c cyme -n "__fish_cyme_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c cyme -n "__fish_cyme_needs_command" -s V -l version -d 'Print version'
complete -c cyme -n "__fish_cyme_needs_command" -f -a "watch" -d 'Watch for USB devices being connected and disconnected'
complete -c cyme -n "__fish_cyme_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c cyme -n "__fish_cyme_using_subcommand watch" -s h -l help -d 'Print help'
complete -c cyme -n "__fish_cyme_using_subcommand help; and not __fish_seen_subcommand_from watch help" -f -a "watch" -d 'Watch for USB devices being connected and disconnected'
complete -c cyme -n "__fish_cyme_using_subcommand help; and not __fish_seen_subcommand_from watch help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
