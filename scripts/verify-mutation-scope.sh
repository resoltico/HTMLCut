#!/usr/bin/env bash

# Proves that cargo-mutants is examining exactly the runtime crates Cargo defines as default members.
set -euo pipefail

if (( $# != 1 )); then
    echo "usage: $0 <cargo-mutants-list-json>" >&2
    exit 64
fi

mutants_json="$1"
if [[ ! -f "$mutants_json" ]]; then
    echo "cargo-mutants list does not exist: $mutants_json" >&2
    exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
metadata_json="$(mktemp "${TMPDIR:-/tmp}/htmlcut-mutation-scope.XXXXXX")"
trap 'rm -f "$metadata_json"' EXIT
cargo metadata --manifest-path "$repo_root/Cargo.toml" --no-deps --format-version 1 > "$metadata_json"

jq -e --slurpfile mutants "$mutants_json" '
  . as $metadata
  | [
      $metadata.packages[]
      | select(.id as $package_id | $metadata.workspace_default_members | index($package_id))
      | {
          name,
          source_prefix: (
            .manifest_path
            | rtrimstr("/Cargo.toml")
            | ltrimstr($metadata.workspace_root + "/")
            + "/src/"
          )
        }
    ] as $default_members
  | ($mutants[0]) as $mutants
  | ($default_members | map(.name) | sort | unique) as $expected_packages
  | ($mutants | map(.package) | sort | unique) as $actual_packages
  | ($expected_packages | length > 0)
    and ($mutants | length > 0)
    and ($actual_packages == $expected_packages)
    and all(
      $mutants[];
      . as $mutant
      | any(
          $default_members[];
          . as $member
          | $member.name == $mutant.package
            and ($mutant.file | startswith($member.source_prefix))
        )
        and (.file | test("/src/(tests/|.*/tests/)") | not)
    )
' "$metadata_json" >/dev/null || {
    echo "cargo-mutants must cover exactly Cargo's default runtime members and their non-test src files" >&2
    exit 1
}
