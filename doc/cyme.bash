_cyme() {
    local i cur prev opts cmds
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="cyme"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        cyme)
            opts="-l -t -d -s -D -v -b -m -F -c -z -h -V --lsusb --tree --vidpid --show --device --filter-name --filter-serial --filter-class --verbose --blocks --bus-blocks --config-blocks --interface-blocks --endpoint-blocks --more --sort-devices --sort-buses --group-devices --hide-buses --hide-hubs --decimal --no-padding --no-colour --ascii --headings --json --from-json --force-libusb --config --debug --mask-serials --gen --help --version"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --vidpid)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --show)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -s)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --device)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -D)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter-name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter-serial)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter-class)
                    COMPREPLY=($(compgen -W "use-interface-descriptor audio cdc-communications hid physical image printer mass-storage hub cdc-data smart-cart content-security video personal-healthcare audio-video billboard usb-type-c-bridge i3c-device diagnostic wireless-controller miscellaneous application-specific-interface vendor-specific-class" -- "${cur}"))
                    return 0
                    ;;
                --blocks)
                    COMPREPLY=($(compgen -W "bus-number device-number branch-position port-path sys-path driver icon vendor-id product-id name manufacturer product-name vendor-name serial speed tree-positions bus-power bus-power-used extra-current-used bcd-device bcd-usb class-code sub-class protocol" -- "${cur}"))
                    return 0
                    ;;
                -b)
                    COMPREPLY=($(compgen -W "bus-number device-number branch-position port-path sys-path driver icon vendor-id product-id name manufacturer product-name vendor-name serial speed tree-positions bus-power bus-power-used extra-current-used bcd-device bcd-usb class-code sub-class protocol" -- "${cur}"))
                    return 0
                    ;;
                --bus-blocks)
                    COMPREPLY=($(compgen -W "bus-number icon name host-controller pci-vendor pci-device pci-revision port-path" -- "${cur}"))
                    return 0
                    ;;
                --config-blocks)
                    COMPREPLY=($(compgen -W "name number num-interfaces attributes icon-attributes max-power" -- "${cur}"))
                    return 0
                    ;;
                --interface-blocks)
                    COMPREPLY=($(compgen -W "name number port-path class-code sub-class protocol alt-setting driver sys-path num-endpoints icon" -- "${cur}"))
                    return 0
                    ;;
                --endpoint-blocks)
                    COMPREPLY=($(compgen -W "number direction transfer-type sync-type usage-type max-packet-size interval" -- "${cur}"))
                    return 0
                    ;;
                --sort-devices)
                    COMPREPLY=($(compgen -W "branch-position device-number no-sort" -- "${cur}"))
                    return 0
                    ;;
                --group-devices)
                    COMPREPLY=($(compgen -W "no-group bus" -- "${cur}"))
                    return 0
                    ;;
                --from-json)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --mask-serials)
                    COMPREPLY=($(compgen -W "hide scramble replace" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

complete -F _cyme -o bashdefault -o default cyme
