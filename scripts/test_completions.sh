#!/usr/bin/env bash
# Sanity-check the generated shell completion files (doc/cyme.bash, doc/_cyme,
# doc/cyme.fish) by sourcing/loading them in each shell and exercising a
# completion lookup, without installing anything into system completion dirs.
#
# Usage: scripts/test_completions.sh [bash] [zsh] [fish]
#   With no arguments, tests whichever of bash/zsh/fish are available.

set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOC_DIR="$ROOT_DIR/doc"

FAIL=0

# Words to drive completion for; each is "<command line> <cursor word>"
TEST_ARGS=(
    "cyme --blocks"
    "cyme -b"
    "cyme --filter-class"
    "cyme --bus-blocks"
)

test_bash() {
    echo "==> Testing bash completion (doc/cyme.bash)"
    local out
    out=$(bash --norc --noprofile -c '
        set -e
        source "'"$DOC_DIR"'/cyme.bash"
        for line in '"$(printf "%q " "${TEST_ARGS[@]}")"'; do
            read -r -a words <<< "$line cursor"
            COMP_WORDS=("${words[@]::${#words[@]}-1}" "")
            COMP_CWORD=$((${#COMP_WORDS[@]}-1))
            COMP_LINE="$line "
            COMP_POINT=${#COMP_LINE}
            _cyme "${COMP_WORDS[0]}" "" "${COMP_WORDS[$((COMP_CWORD-1))]}"
            echo "  $line <TAB> -> ${#COMPREPLY[@]} completions"
        done
    ' 2>&1)
    echo "$out"
    if echo "$out" | grep -qiE "error|not found|bad substitution|unbound variable"; then
        echo "FAIL: bash completion produced an error"
        FAIL=1
    fi
}

test_zsh() {
    echo "==> Testing zsh completion (doc/_cyme)"
    local tmpdump
    tmpdump=$(mktemp -d)
    local out
    out=$(zsh -f -c '
        fpath=("'"$DOC_DIR"'" $fpath)
        autoload -Uz compinit
        compinit -u -d "'"$tmpdump"'/zcompdump"
        autoload -Uz _cyme
        for line in '"$(printf "%q " "${TEST_ARGS[@]}")"'; do
            words=(${=line} "")
            CURRENT=${#words}
            _cyme
            echo "  $line <TAB> -> ok"
        done
    ' 2>&1)
    rm -rf "$tmpdump"
    echo "$out"
    # "_arguments:comparguments:327: can only be called from completion
    # function" is expected/benign: _cyme is being invoked outside of a real
    # completion widget context, but its argument spec is still parsed.
    if echo "$out" | grep -viE "can only be called from completion function" \
        | grep -qiE "error|not found|parse error|command not found"; then
        echo "FAIL: zsh completion produced an error"
        FAIL=1
    fi
}

test_fish() {
    echo "==> Testing fish completion (doc/cyme.fish)"
    local out
    out=$(fish --no-config -c '
        source "'"$DOC_DIR"'/cyme.fish"
        for line in '"$(printf "'%s' " "${TEST_ARGS[@]}")"'
            complete -C"$line " > /dev/null
            echo "  $line <TAB> -> ok"
        end
    ' 2>&1)
    echo "$out"
    if echo "$out" | grep -qiE "error|too many arguments"; then
        echo "FAIL: fish completion produced an error"
        FAIL=1
    fi
}

SHELLS=("$@")
if [[ ${#SHELLS[@]} -eq 0 ]]; then
    SHELLS=(bash zsh fish)
fi

for shell in "${SHELLS[@]}"; do
    if ! command -v "$shell" >/dev/null 2>&1; then
        echo "==> Skipping $shell (not installed)"
        continue
    fi
    case "$shell" in
        bash) test_bash ;;
        zsh) test_zsh ;;
        fish) test_fish ;;
        *) echo "Unknown shell: $shell"; FAIL=1 ;;
    esac
    echo
done

if [[ $FAIL -ne 0 ]]; then
    echo "One or more completion tests FAILED"
    exit 1
fi

echo "All completion tests passed"
