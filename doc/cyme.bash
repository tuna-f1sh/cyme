_cyme() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="cyme"
                ;;
            cyme,help)
                cmd="cyme__help"
                ;;
            cyme,watch)
                cmd="cyme__watch"
                ;;
            cyme__help,help)
                cmd="cyme__help__help"
                ;;
            cyme__help,watch)
                cmd="cyme__help__watch"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        cyme)
            opts="-l -t -d -s -D -v -b -m -F -c -z -h -V --lsusb --tree --vidpid --show --device --filter-name --filter-serial --filter-class --verbose --blocks --bus-blocks --config-blocks --interface-blocks --endpoint-blocks --block-operation --more --sort-devices --sort-buses --group-devices --hide-buses --hide-hubs --list-root-hubs --decimal --no-padding --color --no-color --encoding --ascii --no-icons --icon --headings --json --from-json --force-libusb --config --debug --mask-serials --gen --system-profiler --help --version watch help"
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
                    COMPREPLY=($(compgen -W "use-interface-descriptor audio cdc-communications hid physical image printer mass-storage hub cdc-data smart-card content-security video personal-healthcare audio-video billboard usb-type-c-bridge bdp mctp i3c-device diagnostic wireless-controller miscellaneous application-specific-interface vendor-specific-class" -- "${cur}"))
                    return 0
                    ;;
                --blocks)
                    COMPREPLY=($(compgen -W "bus-number device-number branch-position port-path sys-path driver icon vendor-id product-id vid-pid name manufacturer product-name vendor-name serial speed negotiated-speed tree-positions bus-power bus-power-used extra-current-used bcd-device bcd-usb base-class sub-class protocol uid-class uid-sub-class uid-protocol class base-value last-event event-icon" -- "${cur}"))
                    return 0
                    ;;
                -b)
                    COMPREPLY=($(compgen -W "bus-number device-number branch-position port-path sys-path driver icon vendor-id product-id vid-pid name manufacturer product-name vendor-name serial speed negotiated-speed tree-positions bus-power bus-power-used extra-current-used bcd-device bcd-usb base-class sub-class protocol uid-class uid-sub-class uid-protocol class base-value last-event event-icon" -- "${cur}"))
                    return 0
                    ;;
                --bus-blocks)
                    COMPREPLY=($(compgen -W "bus-number icon name host-controller host-controller-vendor host-controller-device pci-vendor pci-device pci-revision port-path" -- "${cur}"))
                    return 0
                    ;;
                --config-blocks)
                    COMPREPLY=($(compgen -W "name number num-interfaces attributes icon-attributes max-power" -- "${cur}"))
                    return 0
                    ;;
                --interface-blocks)
                    COMPREPLY=($(compgen -W "name number port-path base-class sub-class protocol alt-setting driver sys-path num-endpoints icon uid-class uid-sub-class uid-protocol class base-value" -- "${cur}"))
                    return 0
                    ;;
                --endpoint-blocks)
                    COMPREPLY=($(compgen -W "number direction transfer-type sync-type usage-type max-packet-size interval" -- "${cur}"))
                    return 0
                    ;;
                --block-operation)
                    COMPREPLY=($(compgen -W "add append new prepend remove" -- "${cur}"))
                    return 0
                    ;;
                --sort-devices)
                    COMPREPLY=($(compgen -W "device-number branch-position no-sort" -- "${cur}"))
                    return 0
                    ;;
                --group-devices)
                    COMPREPLY=($(compgen -W "no-group bus" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --encoding)
                    COMPREPLY=($(compgen -W "glyphs utf8 ascii" -- "${cur}"))
                    return 0
                    ;;
                --icon)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
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
        cyme__help)
            opts="watch help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        cyme__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        cyme__help__watch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        cyme__watch)
            opts="-h --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _cyme -o nosort -o bashdefault -o default cyme
else
    complete -F _cyme -o bashdefault -o default cyme
fi
