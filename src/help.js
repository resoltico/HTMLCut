import { EXIT_CODES } from './errors.js';
import { DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES, formatByteSize } from './source.js';

export function renderHelp(version) {
    return `HTMLCut v${version}

Usage:
  htmlcut <input> --from <pattern> --to <pattern> [options]

Arguments:
  <input>                       URL, file path, or - for stdin

Pattern Options:
  -f, --from <pattern>          Start delimiter
  -t, --to <pattern>            End delimiter
  -p, --pattern <mode>          literal | regex (default: literal)
      --flags <flags>           JavaScript RegExp flags, without g (default: u)
  -a, --all                     Return every non-overlapping match
  -c, --capture <mode>          inner | outer (default: inner)

Output Options:
  -F, --format <format>         text | html | json | none (default: text)
  -o, --bundle <dir>            Write selection.html, selection.txt, report.json
  -b, --base-url <url>          Resolve relative links against this absolute URL

Limits:
      --max-bytes <size>        Limit input size (default: ${formatByteSize(DEFAULT_MAX_BYTES)})
      --fetch-timeout-ms <ms>   URL fetch timeout (default: ${DEFAULT_FETCH_TIMEOUT_MS})

Misc:
  -v, --verbose                 Print bundle/status info to stderr
  -V, --version                 Print version
  -h, --help                    Show this help

Exit Codes:
  ${EXIT_CODES.INTERNAL}  Unexpected internal error
  ${EXIT_CODES.USAGE}  Invalid usage or invalid patterns
  ${EXIT_CODES.SOURCE}  Input could not be read or exceeded limits
  ${EXIT_CODES.EXTRACTION}  No match or incomplete match
  ${EXIT_CODES.OUTPUT}  Bundle files could not be written

Examples:
  htmlcut https://example.com --from '<article>' --to '</article>'
  curl -sL https://example.com | htmlcut - --from '<h2>' --to '</h2>' --all --format json
  htmlcut ./page.html --from '<main>' --to '</main>' --capture outer --bundle ./cut --format none
`;
}
