#!/bin/bash
# Cargo invokes this as: rustc-with-icon.sh <rustc> <rustc args...>
rustc="$1"
shift
"$rustc" "$@"
status=$?
if [[ $status -ne 0 || "$(uname)" != "Darwin" ]]; then
    exit "$status"
fi

bin_name="${CARGO_BIN_NAME:-}"
case "$bin_name" in
    Sam | Sam-tui | Sam_tui) ;;
    *) exit 0 ;;
esac

expanded=()
for arg in "$@"; do
    if [[ "$arg" == @* && -f "${arg:1}" ]]; then
        while IFS= read -r line || [[ -n "$line" ]]; do
            expanded+=("$line")
        done <"${arg:1}"
    else
        expanded+=("$arg")
    fi
done

out=""
out_dir=""
extra=""
previous=""
for arg in "${expanded[@]}"; do
    if [[ "$previous" == "-o" ]]; then
        out="$arg"
    elif [[ "$previous" == "--out-dir" ]]; then
        out_dir="$arg"
    fi
    case "$arg" in
        extra-filename=*) extra="${arg#extra-filename=}" ;;
        -Cextra-filename=*) extra="${arg#-Cextra-filename=}" ;;
    esac
    previous="$arg"
done
if [[ -z "$out" && -n "$out_dir" && -n "$extra" ]]; then
    out="$out_dir/${CARGO_CRATE_NAME:-$bin_name}${extra}"
fi
case "$out" in
    *.rmeta | *.rlib | *.d | *.o | "") exit 0 ;;
esac
if [[ ! -f "$out" ]]; then
    exit 0
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
icon="$root/assets/icon_256.png"
tool="$root/target/set-icon"
if [[ ! -x "$tool" || "$root/scripts/set-icon.swift" -nt "$tool" ]]; then
    mkdir -p "$root/target"
    swiftc -O -o "$tool" "$root/scripts/set-icon.swift" || exit 0
fi

"$tool" "$out" "$icon" || true

# Cargo hard-links this file to target/<profile>/<bin> after the wrapper returns.
final="$(dirname "$(dirname "$out")")/$bin_name"
if [[ "$final" == "$out" ]]; then
    exit 0
fi
inode="$(stat -f %i "$out")"
(
    for _ in $(seq 1 40); do
        if [[ -f "$final" ]]; then
            final_inode="$(stat -f %i "$final" 2>/dev/null || echo 0)"
            if [[ "$final_inode" == "$inode" || ! "$out" -nt "$final" ]]; then
                "$tool" "$final" "$icon" || true
                exit 0
            fi
        fi
        sleep 0.25
    done
) >/dev/null 2>&1 &
exit 0
