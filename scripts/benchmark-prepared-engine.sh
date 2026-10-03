#!/usr/bin/env bash
# Record the operational effect of the prepared engine against the immutable v16 one-shot workflow.

set -euo pipefail

readonly benchmark_ref="v16.0.0"
readonly plan_count=50
readonly document_article_count=1024

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
. "${script_dir}/common.sh"
script_dir="$(htmlcut_resolve_script_dir "${BASH_SOURCE[0]}")"
readonly script_dir
repo_root="$(htmlcut_repo_root_from_script_dir "${script_dir}")"
readonly repo_root

if (( $# != 2 )); then
    printf 'usage: %s <published-v16-binary> <report-path>\n' "$0" >&2
    exit 2
fi

[[ -z "$(git -C "${repo_root}" status --porcelain)" ]] || htmlcut_die "benchmark source must be clean"
one_shot_binary="$(cd -- "$(dirname -- "$1")" && pwd)/$(basename -- "$1")"
readonly one_shot_binary
[[ "$("${one_shot_binary}" --version)" == 'htmlcut 16.0.0' ]] || htmlcut_die "baseline must be the published 16.0.0 native executable"
report_path="$2"
readonly report_path
mkdir -p "$(dirname -- "${report_path}")"
workspace_dir="$(mktemp -d -t htmlcut-prepared-workflow-XXXXXX)"
readonly workspace_dir
trap 'rm -rf -- "${workspace_dir}"' EXIT

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
[[ -x "${prepared_binary}" ]] || htmlcut_die "missing prepared-engine benchmark ${prepared_binary}"

one_shot_runner="${workspace_dir}/run-one_shot.sh"
# shellcheck disable=SC2016 # The generated runner, not this script, expands these variables.
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    'for (( index = 0; index < plan_count; index += 1 )); do' \
    '  value="$("${one_shot_binary}" extract --file "${benchmark_html}" --css "article[data-benchmark-index=\"${index}\"]" --raw)"' \
    '  [[ "$value" == "Headline ${index}Prepared engine benchmark content." ]]' \
    '  printf "%s\n" "$value"' \
    'done' >"${one_shot_runner}"
chmod +x "${one_shot_runner}"

export workspace_dir plan_count one_shot_binary benchmark_html
python3 - "${repo_root}" "${prepared_binary}" "${one_shot_runner}" "${report_path}" "${benchmark_ref}" <<'PYTHON'
import hashlib,json,platform,subprocess,sys
from pathlib import Path
repo,prepared,baseline,report,baseline_ref=sys.argv[1:]
sys.path.insert(0,str(Path(repo)/'evaluation'))
from measurements import paired
expected=[f'Headline {index}Prepared engine benchmark content.' for index in range(50)]
def decode(name,data):
    if name=='one_shot': return data.decode().splitlines()
    value=json.loads(data)
    assert value['prepared_document_count']==1 and value['full_document_parse_count']==1
    assert value['compiled_plan_count']==50 and value['execution_count']==50
    assert value['selected_match_count']==50
    return value['values']
measurements,outputs=paired({'prepared':[prepared],'one_shot':['bash',baseline]},expected,decode,
                            rss_directory=Path(report).parent/'prepared-workflow-rss')
result=dict(benchmark='htmlcut.prepared_engine@3',baseline_ref=baseline_ref,platform=platform.platform(),
            source_commit=subprocess.check_output(['git','-C',repo,'rev-parse','HEAD']).decode().strip(),
            prepared_binary_sha256=hashlib.sha256(Path(prepared).read_bytes()).hexdigest(),
            baseline_binary_sha256=hashlib.sha256(Path(__import__('os').environ['one_shot_binary']).read_bytes()).hexdigest(),
            fixture_sha256=hashlib.sha256(Path(__import__('os').environ['benchmark_html']).read_bytes()).hexdigest(),
            values=expected,measurements=measurements,
            prepared_model=json.loads(outputs['prepared']),
            notes=['Complete outputs checked on every randomized launch; 3 warmups and15 measured trials.',
                   'Baseline: fifty sequential native16 CLI launches with Bash coordination; prepared: one current process, one parse, fifty compiled plans.',
                   'This workflow comparison includes different versions/process coordination; it is not a pure parser-engine speed comparison.',
                   'RSS is a separate checked operational observation, not a correctness threshold.'])
Path(report).write_text(json.dumps(result,indent=2)+'\n')
PYTHON
printf 'Prepared-engine benchmark recorded at %s\n' "${report_path}"
