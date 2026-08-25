#!/usr/bin/env bash
set -euo pipefail

readonly shard_total=16

jq -cn --argjson shard_total "$shard_total" '[
    range(0; $shard_total) as $shard
    | {
        selector: "\($shard)/\($shard_total)",
        artifact_name: "cargo-mutants-shard-\($shard)-of-\($shard_total)"
      }
]'
