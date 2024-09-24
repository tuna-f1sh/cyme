
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'cyme' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'cyme'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'cyme' {
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Show only devices with the specified vendor and product ID numbers (in hexadecimal) in format VID:[PID]')
            [CompletionResult]::new('--vidpid', 'vidpid', [CompletionResultType]::ParameterName, 'Show only devices with the specified vendor and product ID numbers (in hexadecimal) in format VID:[PID]')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Show only devices with specified device and/or bus numbers (in decimal) in format [[bus]:][devnum]')
            [CompletionResult]::new('--show', 'show', [CompletionResultType]::ParameterName, 'Show only devices with specified device and/or bus numbers (in decimal) in format [[bus]:][devnum]')
            [CompletionResult]::new('-D', 'D ', [CompletionResultType]::ParameterName, 'Selects which device lsusb will examine - supplied as Linux /dev/bus/usb/BBB/DDD style path')
            [CompletionResult]::new('--device', 'device', [CompletionResultType]::ParameterName, 'Selects which device lsusb will examine - supplied as Linux /dev/bus/usb/BBB/DDD style path')
            [CompletionResult]::new('--filter-name', 'filter-name', [CompletionResultType]::ParameterName, 'Filter on string contained in name')
            [CompletionResult]::new('--filter-serial', 'filter-serial', [CompletionResultType]::ParameterName, 'Filter on string contained in serial')
            [CompletionResult]::new('--filter-class', 'filter-class', [CompletionResultType]::ParameterName, 'Filter on USB class code')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each device and in what order')
            [CompletionResult]::new('--blocks', 'blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each device and in what order')
            [CompletionResult]::new('--bus-blocks', 'bus-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each bus and in what order')
            [CompletionResult]::new('--config-blocks', 'config-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each configuration and in what order')
            [CompletionResult]::new('--interface-blocks', 'interface-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each interface and in what order')
            [CompletionResult]::new('--endpoint-blocks', 'endpoint-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each endpoint and in what order')
            [CompletionResult]::new('--sort-devices', 'sort-devices', [CompletionResultType]::ParameterName, 'Sort devices operation')
            [CompletionResult]::new('--group-devices', 'group-devices', [CompletionResultType]::ParameterName, 'Group devices by value when listing')
            [CompletionResult]::new('--color', 'color', [CompletionResultType]::ParameterName, 'Output coloring mode')
            [CompletionResult]::new('--encoding', 'encoding', [CompletionResultType]::ParameterName, 'Output character encoding')
            [CompletionResult]::new('--icon', 'icon', [CompletionResultType]::ParameterName, 'When to print icon blocks')
            [CompletionResult]::new('--from-json', 'from-json', [CompletionResultType]::ParameterName, 'Read from json output rather than profiling system')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Path to user config file to use for custom icons, colours and default settings')
            [CompletionResult]::new('--config', 'config', [CompletionResultType]::ParameterName, 'Path to user config file to use for custom icons, colours and default settings')
            [CompletionResult]::new('--mask-serials', 'mask-serials', [CompletionResultType]::ParameterName, 'Mask serial numbers with ''*'' or random chars')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Attempt to maintain compatibility with lsusb output')
            [CompletionResult]::new('--lsusb', 'lsusb', [CompletionResultType]::ParameterName, 'Attempt to maintain compatibility with lsusb output')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Dump USB device hierarchy as a tree')
            [CompletionResult]::new('--tree', 'tree', [CompletionResultType]::ParameterName, 'Dump USB device hierarchy as a tree')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Verbosity level: 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and all blocks')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Verbosity level: 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and all blocks')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'Print more blocks by default at each verbosity')
            [CompletionResult]::new('--more', 'more', [CompletionResultType]::ParameterName, 'Print more blocks by default at each verbosity')
            [CompletionResult]::new('--sort-buses', 'sort-buses', [CompletionResultType]::ParameterName, 'Sort devices by bus number. If using any sort-devices other than no-sort, this happens automatically')
            [CompletionResult]::new('--hide-buses', 'hide-buses', [CompletionResultType]::ParameterName, 'Hide empty buses when printing tree; those with no devices')
            [CompletionResult]::new('--hide-hubs', 'hide-hubs', [CompletionResultType]::ParameterName, 'Hide empty hubs when printing tree; those with no devices. When listing will hide hubs regardless of whether empty of not')
            [CompletionResult]::new('--list-root-hubs', 'list-root-hubs', [CompletionResultType]::ParameterName, 'Show root hubs when listing; Linux only')
            [CompletionResult]::new('--decimal', 'decimal', [CompletionResultType]::ParameterName, 'Show base16 values as base10 decimal instead')
            [CompletionResult]::new('--no-padding', 'no-padding', [CompletionResultType]::ParameterName, 'Disable padding to align blocks - will cause --headings to become maligned')
            [CompletionResult]::new('--no-color', 'no-color', [CompletionResultType]::ParameterName, 'Disable coloured output, can also use NO_COLOR environment variable')
            [CompletionResult]::new('--ascii', 'ascii', [CompletionResultType]::ParameterName, 'Disables icons and utf-8 characters')
            [CompletionResult]::new('--no-icons', 'no-icons', [CompletionResultType]::ParameterName, 'Disables all Block icons by not using any IconTheme. Providing custom XxxxBlocks without any icons is a nicer way to do this')
            [CompletionResult]::new('--headings', 'headings', [CompletionResultType]::ParameterName, 'Show block headings')
            [CompletionResult]::new('--json', 'json', [CompletionResultType]::ParameterName, 'Output as json format after sorting, filters and tree settings are applied; without -tree will be flattened dump of devices')
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'Force libusb profiler on macOS rather than using/combining system_profiler output')
            [CompletionResult]::new('--force-libusb', 'force-libusb', [CompletionResultType]::ParameterName, 'Force libusb profiler on macOS rather than using/combining system_profiler output')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE')
            [CompletionResult]::new('--debug', 'debug', [CompletionResultType]::ParameterName, 'Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE')
            [CompletionResult]::new('--gen', 'gen', [CompletionResultType]::ParameterName, 'Generate cli completions and man page')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
