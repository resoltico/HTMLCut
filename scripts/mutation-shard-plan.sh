#!/usr/bin/env bash
set -euo pipefail

readonly default_shard_total=16

if (( $# > 1 )); then
    printf 'usage: %s [shard-count]\n' "$0" >&2
    exit 1
fi

shard_total="${1:-${default_shard_total}}"
if ! [[ "${shard_total}" =~ ^[1-9][0-9]*$ ]]; then
    printf 'shard count must be a positive integer: %s\n' "${shard_total}" >&2
    exit 1
fi

jq -cn --argjson shard_total "$shard_total" '[
    range(0; $shard_total) as $shard
    | {
        selector: "\($shard)/\($shard_total)",
        artifact_name: "cargo-mutants-shard-\($shard)-of-\($shard_total)"
      }
]'
