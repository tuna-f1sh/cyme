
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
            [CompletionResult]::new('-D', 'D', [CompletionResultType]::ParameterName, 'Selects which device lsusb will examine - supplied as Linux /dev/bus/usb/BBB/DDD style path')
            [CompletionResult]::new('--device', 'device', [CompletionResultType]::ParameterName, 'Selects which device lsusb will examine - supplied as Linux /dev/bus/usb/BBB/DDD style path')
            [CompletionResult]::new('--filter-name', 'filter-name', [CompletionResultType]::ParameterName, 'Filter on string contained in name')
            [CompletionResult]::new('--filter-serial', 'filter-serial', [CompletionResultType]::ParameterName, 'Filter on string contained in serial')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each device and in what order')
            [CompletionResult]::new('--blocks', 'blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each device and in what order')
            [CompletionResult]::new('--bus-blocks', 'bus-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each bus and in what order')
            [CompletionResult]::new('--config-blocks', 'config-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each configuration and in what order')
            [CompletionResult]::new('--interface-blocks', 'interface-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each interface and in what order')
            [CompletionResult]::new('--endpoint-blocks', 'endpoint-blocks', [CompletionResultType]::ParameterName, 'Specify the blocks which will be displayed for each endpoint and in what order')
            [CompletionResult]::new('--sort-devices', 'sort-devices', [CompletionResultType]::ParameterName, 'Sort devices by value')
            [CompletionResult]::new('--group-devices', 'group-devices', [CompletionResultType]::ParameterName, 'Group devices by value when listing')
            [CompletionResult]::new('--from-json', 'from-json', [CompletionResultType]::ParameterName, 'Read from json output rather than profiling system - must use --tree json dump')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Path to user config file to use for custom icons, colours and default settings')
            [CompletionResult]::new('--config', 'config', [CompletionResultType]::ParameterName, 'Path to user config file to use for custom icons, colours and default settings')
            [CompletionResult]::new('--hide-serials', 'hide-serials', [CompletionResultType]::ParameterName, 'Hide serial numbers')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Attempt to maintain compatibility with lsusb output')
            [CompletionResult]::new('--lsusb', 'lsusb', [CompletionResultType]::ParameterName, 'Attempt to maintain compatibility with lsusb output')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Dump USB device hierarchy as a tree')
            [CompletionResult]::new('--tree', 'tree', [CompletionResultType]::ParameterName, 'Dump USB device hierarchy as a tree')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Verbosity level: 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and all blocks')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Verbosity level: 1 prints device configurations; 2 prints interfaces; 3 prints interface endpoints; 4 prints everything and all blocks')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'Print more blocks by default at each verbosity')
            [CompletionResult]::new('--more', 'more', [CompletionResultType]::ParameterName, 'Print more blocks by default at each verbosity')
            [CompletionResult]::new('--sort-buses', 'sort-buses', [CompletionResultType]::ParameterName, 'Sort devices by bus number')
            [CompletionResult]::new('--hide-buses', 'hide-buses', [CompletionResultType]::ParameterName, 'Hide empty buses; those with no devices')
            [CompletionResult]::new('--hide-hubs', 'hide-hubs', [CompletionResultType]::ParameterName, 'Hide empty hubs; those with no devices')
            [CompletionResult]::new('--decimal', 'decimal', [CompletionResultType]::ParameterName, 'Show base16 values as base10 decimal instead')
            [CompletionResult]::new('--no-padding', 'no-padding', [CompletionResultType]::ParameterName, 'Disable padding to align blocks')
            [CompletionResult]::new('--no-colour', 'no-colour', [CompletionResultType]::ParameterName, 'Disable coloured output, can also use NO_COLOR environment variable')
            [CompletionResult]::new('--ascii', 'ascii', [CompletionResultType]::ParameterName, 'Disables icons and utf-8 charactors')
            [CompletionResult]::new('--headings', 'headings', [CompletionResultType]::ParameterName, 'Show block headings')
            [CompletionResult]::new('--json', 'json', [CompletionResultType]::ParameterName, 'Output as json format after sorting, filters and tree settings are applied; without -tree will be flattened dump of devices')
            [CompletionResult]::new('-F', 'F', [CompletionResultType]::ParameterName, 'Force libusb profiler on macOS rather than using/combining system_profiler output')
            [CompletionResult]::new('--force-libusb', 'force-libusb', [CompletionResultType]::ParameterName, 'Force libusb profiler on macOS rather than using/combining system_profiler output')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE')
            [CompletionResult]::new('--debug', 'debug', [CompletionResultType]::ParameterName, 'Turn debugging information on. Alternatively can use RUST_LOG env: INFO, DEBUG, TRACE')
            [CompletionResult]::new('--gen', 'gen', [CompletionResultType]::ParameterName, 'Generate cli completions and man page')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information (use `--help` for more detail)')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information (use `--help` for more detail)')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
