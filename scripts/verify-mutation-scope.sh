#!/usr/bin/env bash

# Proves the complete mutation inventory covers the maintained source scope, or that a diff
# inventory is a subset of that already-verified complete inventory.
set -euo pipefail

if [[ "${1:-}" == "--subset" ]]; then
    if (( $# != 3 )); then
        echo "usage: $0 --subset <diff-mutants-json> <full-mutants-json>" >&2
        exit 64
    fi
    subset_json="$2"
    full_json="$3"
    if [[ ! -f "$subset_json" || ! -f "$full_json" ]]; then
        echo "mutation subset or full inventory does not exist" >&2
        exit 1
    fi
    jq -e --slurpfile full "$full_json" '
      ($full[0] | map({name, package, file})) as $full_identities
      | all(.[]; {name, package, file} as $identity
          | $full_identities | index($identity) != null)
    ' "$subset_json" >/dev/null || {
        echo "pull-request mutation inventory contains an identity outside the verified full scope" >&2
        exit 1
    }
    exit 0
fi

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
  | [
      { name: "xtask", source_prefix: "xtask/src/" }
    ] as $tooling_members
  | [
      { package: "htmlcut-selectors", file: "patches/rust/selectors/work_budget.rs" },
      { package: "htmlcut-scraper", file: "patches/rust/scraper/src/html/clone.rs" },
      { package: "htmlcut-scraper", file: "patches/rust/scraper/src/selector/budget.rs" }
    ] as $fork_files
  | ($mutants[0]) as $mutants
  | (($default_members | map(.name)) + ($tooling_members | map(.name)) + ($fork_files | map(.package)) | sort | unique) as $expected_packages
  | ($mutants | map(.package) | sort | unique) as $actual_packages
  | ($expected_packages | length > 0)
    and ($mutants | length > 0)
    and ($actual_packages == $expected_packages)
    and all(
      $fork_files[];
      . as $fork
      | any(
          $mutants[];
          .package == $fork.package and .file == $fork.file
        )
    )
        and all(
      $mutants[];
      . as $mutant
      | any(
          $default_members[];
          . as $member
          | $member.name == $mutant.package
            and ($mutant.file | startswith($member.source_prefix))
        )
        or any(
          $tooling_members[];
          . as $member
          | $member.name == $mutant.package
            and ($mutant.file | startswith($member.source_prefix))
        )
        and (.file | test("/src/(tests/|.*/tests/)") | not)
        or any(
          $fork_files[];
          . as $fork
          | $mutant.package == $fork.package
            and $mutant.file == $fork.file
        )
    )
' "$metadata_json" >/dev/null || {
    echo "cargo-mutants must cover default runtime members, maintainer tooling, and the exact maintained selector and scraper boundary files" >&2
    exit 1
}
