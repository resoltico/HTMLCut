#!/usr/bin/env bash
# Parse peak resident memory from the host's /usr/bin/time output as bytes.

htmlcut_peak_rss_bytes() {
    local platform="$1"
    local time_output="$2"
    local value
    case "${platform}" in
        Darwin)
            value="$(sed -n -E \
                's/^[[:space:]]*([0-9]+)[[:space:]]+maximum resident set size$/\1/p' \
                "${time_output}")"
            ;;
        Linux)
            value="$(sed -n -E \
                's/^.*Maximum resident set size \(kbytes\):[[:space:]]*([0-9]+)[[:space:]]*$/\1/p' \
                "${time_output}")"
            ;;
        *)
            htmlcut_die "unsupported benchmark time platform ${platform}"
            ;;
    esac
    [[ "${value}" =~ ^[0-9]+$ ]] || htmlcut_die \
        "could not read exactly one peak resident memory value from ${time_output}"
    if [[ "${platform}" == Linux ]]; then
        (( ${#value} <= 15 )) || htmlcut_die "benchmark peak resident memory is out of range"
        value="$(( value * 1024 ))"
    fi
    printf '%s\n' "${value}"
}
