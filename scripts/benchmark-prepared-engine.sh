#!/usr/bin/env bash
# Record the operational effect of the prepared engine against the immutable v16 one-shot workflow.

set -euo pipefail

readonly benchmark_ref="v16.0.0"
readonly plan_count=50
readonly document_article_count=1024

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
# shellcheck source=scripts/benchmark-time.sh
. "${script_dir}/benchmark-time.sh"
script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
readonly script_dir
repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
readonly repo_root

if (( $# > 1 )); then
    printf 'usage: %s [report-path]\n' "$0" >&2
    exit 2
fi

mkdir -p "${repo_root}/tmp"
workspace_dir="$(mktemp -d "${repo_root}/tmp/prepared-engine-benchmark.XXXXXX")"
readonly workspace_dir
baseline_dir="${workspace_dir}/one_shot"
readonly baseline_dir

cleanup() {
    if [[ -d "${baseline_dir}/.git" || -f "${baseline_dir}/.git" ]]; then
        git -C "${repo_root}" worktree remove --force "${baseline_dir}"
    fi
    rm -rf -- "${workspace_dir}"
}
trap cleanup EXIT

report_path="${1:-${repo_root}/tmp/prepared-engine-benchmark-report.json}"
readonly report_path
case "${report_path}" in
    "${workspace_dir}"/*)
        htmlcut_die "benchmark report path must be outside the disposable workspace"
        ;;
esac
mkdir -p "$(dirname -- "${report_path}")"

case "$(uname -s)" in
    Darwin)
        readonly time_command="/usr/bin/time"
        time_args=(-l)
        ;;
    Linux)
        readonly time_command="/usr/bin/time"
        time_args=(-v)
        ;;
    *)
        printf 'prepared-engine benchmark requires a supported /usr/bin/time implementation\n' >&2
        exit 1
        ;;
esac

benchmark_html="${workspace_dir}/benchmark.html"
{
    printf '<!doctype html><html><body>'
    for (( index = 0; index < document_article_count; index += 1 )); do
        if (( index < plan_count )); then
            printf '<article data-benchmark-index="%s"><h1>Headline %s</h1><p>Prepared engine benchmark content.</p></article>' "$index" "$index"
        else
            printf '<article><h1>Background %s</h1><p>Prepared engine benchmark content.</p></article>' "$index"
        fi
    done
    printf '</body></html>'
} >"${benchmark_html}"

prepared_target_dir="${workspace_dir}/prepared-target"
prepared_build_dir="${workspace_dir}/prepared-build"
CARGO_TARGET_DIR="${prepared_target_dir}" CARGO_BUILD_BUILD_DIR="${prepared_build_dir}" \
    cargo build --quiet --locked --release -p htmlcut-core --example prepared_engine_benchmark
prepared_binary="${prepared_target_dir}/release/examples/prepared_engine_benchmark$(htmlcut_host_executable_suffix)"
[[ -x "${prepared_binary}" ]] || htmlcut_die "missing prepared prepared-engine benchmark ${prepared_binary}"

prepared_output="${workspace_dir}/prepared.json"
prepared_time="${workspace_dir}/prepared.time"
"${time_command}" "${time_args[@]}" "${prepared_binary}" >"${prepared_output}" 2>"${prepared_time}"
jq -e \
    --argjson plan_count "${plan_count}" \
    '.workflow == "prepared_engine"
        and .prepared_document_count == 1
        and .full_document_parse_count == 1
        and .compiled_plan_count == $plan_count
        and .execution_count == $plan_count
        and .selected_match_count == $plan_count' \
    "${prepared_output}" >/dev/null

git -C "${repo_root}" worktree add --quiet --detach "${baseline_dir}" "${benchmark_ref}"
one_shot_target_dir="${workspace_dir}/one_shot-target"
one_shot_build_dir="${workspace_dir}/one_shot-build"
(
    cd "${baseline_dir}"
    CARGO_TARGET_DIR="${one_shot_target_dir}" CARGO_BUILD_BUILD_DIR="${one_shot_build_dir}" \
        cargo build --quiet --locked --release -p htmlcut-cli
)
one_shot_binary="${one_shot_target_dir}/release/htmlcut$(htmlcut_host_executable_suffix)"
[[ -x "${one_shot_binary}" ]] || htmlcut_die "missing one_shot CLI benchmark ${one_shot_binary}"

one_shot_runner="${workspace_dir}/run-one_shot.sh"
# shellcheck disable=SC2016 # The generated runner, not this script, expands these variables.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'for (( index = 0; index < plan_count; index += 1 )); do' \
    '  "${one_shot_binary}" extract --file "${benchmark_html}" --css "article[data-benchmark-index=\"${index}\"]" --raw >"${workspace_dir}/baseline-value"' \
    '  [[ "$(cat "${workspace_dir}/baseline-value")" == "Headline ${index}Prepared engine benchmark content." ]]' \
    'done' >"${one_shot_runner}"
chmod +x "${one_shot_runner}"

one_shot_time="${workspace_dir}/one_shot.time"
env workspace_dir="${workspace_dir}" plan_count="${plan_count}" one_shot_binary="${one_shot_binary}" benchmark_html="${benchmark_html}" \
    "${time_command}" "${time_args[@]}" "${one_shot_runner}" >/dev/null 2>"${one_shot_time}"

one_shot_peak_rss_bytes="$(htmlcut_peak_rss_bytes "$(uname -s)" "${one_shot_time}")"
prepared_peak_rss_bytes="$(htmlcut_peak_rss_bytes "$(uname -s)" "${prepared_time}")"
jq -n \
    --arg benchmark_ref "${benchmark_ref}" \
    --arg platform "$(uname -s)" \
    --argjson plan_count "${plan_count}" \
    --argjson document_article_count "${document_article_count}" \
    --argjson one_shot_peak_rss_bytes "${one_shot_peak_rss_bytes}" \
    --argjson prepared_peak_rss_bytes "${prepared_peak_rss_bytes}" \
    --slurpfile prepared "${prepared_output}" \
    '{
        benchmark: "htmlcut.prepared_engine@3",
        baseline_ref: $benchmark_ref,
        platform: $platform,
        document_article_count: $document_article_count,
        plan_count: $plan_count,
        baseline: {
            workflow: "one_shot_one_shot_cli",
            full_document_parse_count: $plan_count,
            peak_rss_bytes: $one_shot_peak_rss_bytes
        },
        prepared: ($prepared[0] + {peak_rss_bytes: $prepared_peak_rss_bytes}),
        notes: [
            "The one_shot baseline executes one equivalent CSS extraction in a fresh CLI process for each plan.",
            "The prepared run prepares one immutable document and executes fifty compiled plans in one process.",
            "Peak RSS is an operational observation, not a correctness threshold."
        ]
    }' >"${report_path}"

printf 'Prepared-engine benchmark recorded at %s\n' "${report_path}"
