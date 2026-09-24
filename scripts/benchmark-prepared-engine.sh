#!/usr/bin/env bash
# Record the operational effect of HTMLCut 14's prepared engine against the immutable v13 workflow.

set -euo pipefail

readonly benchmark_ref="v13.2.0"
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
baseline_dir="${workspace_dir}/v13"
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

v14_target_dir="${workspace_dir}/v14-target"
v14_build_dir="${workspace_dir}/v14-build"
CARGO_TARGET_DIR="${v14_target_dir}" CARGO_BUILD_BUILD_DIR="${v14_build_dir}" \
    cargo build --quiet --locked --release -p htmlcut-core --example prepared_engine_benchmark
v14_binary="${v14_target_dir}/release/examples/prepared_engine_benchmark$(htmlcut_host_executable_suffix)"
[[ -x "${v14_binary}" ]] || htmlcut_die "missing v14 prepared-engine benchmark ${v14_binary}"

v14_output="${workspace_dir}/v14.json"
v14_time="${workspace_dir}/v14.time"
"${time_command}" "${time_args[@]}" "${v14_binary}" >"${v14_output}" 2>"${v14_time}"
jq -e \
    --argjson plan_count "${plan_count}" \
    '.workflow == "prepared_engine"
        and .prepared_document_count == 1
        and .full_document_parse_count == 1
        and .compiled_plan_count == $plan_count
        and .execution_count == $plan_count
        and .selected_match_count == $plan_count' \
    "${v14_output}" >/dev/null

git -C "${repo_root}" worktree add --quiet --detach "${baseline_dir}" "${benchmark_ref}"
v13_target_dir="${workspace_dir}/v13-target"
v13_build_dir="${workspace_dir}/v13-build"
(
    cd "${baseline_dir}"
    CARGO_TARGET_DIR="${v13_target_dir}" CARGO_BUILD_BUILD_DIR="${v13_build_dir}" \
        cargo build --quiet --locked --release -p htmlcut-cli
)
v13_binary="${v13_target_dir}/release/htmlcut$(htmlcut_host_executable_suffix)"
[[ -x "${v13_binary}" ]] || htmlcut_die "missing v13 CLI benchmark ${v13_binary}"

v13_runner="${workspace_dir}/run-v13.sh"
# shellcheck disable=SC2016 # The generated runner, not this script, expands these variables.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'for (( index = 0; index < plan_count; index += 1 )); do' \
    '  "${v13_binary}" select "${benchmark_html}" --css "article[data-benchmark-index=\"${index}\"]" >/dev/null' \
    'done' >"${v13_runner}"
chmod +x "${v13_runner}"

v13_time="${workspace_dir}/v13.time"
env plan_count="${plan_count}" v13_binary="${v13_binary}" benchmark_html="${benchmark_html}" \
    "${time_command}" "${time_args[@]}" "${v13_runner}" >/dev/null 2>"${v13_time}"

v13_peak_rss_bytes="$(htmlcut_peak_rss_bytes "$(uname -s)" "${v13_time}")"
v14_peak_rss_bytes="$(htmlcut_peak_rss_bytes "$(uname -s)" "${v14_time}")"
jq -n \
    --arg benchmark_ref "${benchmark_ref}" \
    --arg platform "$(uname -s)" \
    --argjson plan_count "${plan_count}" \
    --argjson document_article_count "${document_article_count}" \
    --argjson v13_peak_rss_bytes "${v13_peak_rss_bytes}" \
    --argjson v14_peak_rss_bytes "${v14_peak_rss_bytes}" \
    --slurpfile v14 "${v14_output}" \
    '{
        benchmark: "htmlcut.prepared_engine@2",
        baseline_ref: $benchmark_ref,
        platform: $platform,
        document_article_count: $document_article_count,
        plan_count: $plan_count,
        baseline: {
            workflow: "v13_one_shot_cli",
            full_document_parse_count: $plan_count,
            peak_rss_bytes: $v13_peak_rss_bytes
        },
        prepared: ($v14[0] + {peak_rss_bytes: $v14_peak_rss_bytes}),
        notes: [
            "The v13 baseline executes one equivalent CSS extraction in a fresh CLI process for each plan.",
            "The v14 run prepares one immutable document and executes fifty compiled plans in one process.",
            "Peak RSS is an operational observation, not a correctness threshold."
        ]
    }' >"${report_path}"

printf 'Prepared-engine benchmark recorded at %s\n' "${report_path}"
