use crate::metadata::TOOL_NAME;

pub(crate) fn raw_args_prefers_json(raw_args: &[String]) -> bool {
    RawInvocation::parse(raw_args).prefers_json_errors()
}

pub(crate) fn raw_args_requests_version(raw_args: &[String]) -> bool {
    let invocation = RawInvocation::parse(raw_args);
    invocation.command_tokens.is_empty()
        && root_option_tokens_are_known(raw_args)
        && raw_option_tokens(raw_args).any(token_requests_version)
}

pub(crate) fn raw_args_requests_help(raw_args: &[String]) -> bool {
    raw_option_tokens(raw_args).any(token_requests_help)
}

pub(crate) fn command_name_from_raw_args(raw_args: &[String]) -> String {
    RawInvocation::parse(raw_args).command_name()
}

pub(crate) fn clap_error_message(error: &clap::Error) -> String {
    let rendered = error.to_string();
    if !rendered.trim_start().starts_with("error:") {
        return rendered.trim().to_owned();
    }
    let suggests_help = rendered
        .lines()
        .any(|line| line.trim_start().starts_with("For more information, try"));
    let mut lines = rendered
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let primary_line = lines.next().unwrap_or("error:");
    let primary = primary_line
        .strip_prefix("error: ")
        .or_else(|| primary_line.strip_prefix("error:"))
        .unwrap_or(primary_line)
        .trim()
        .to_owned();

    let mut detail_lines = Vec::new();
    for line in lines {
        if line.starts_with("Usage: ") {
            break;
        }
        detail_lines.push(line.to_owned());
    }

    let mut message = primary;
    if !detail_lines.is_empty() {
        if !message.ends_with(':') {
            message.push(':');
        }
        message.push(' ');
        message.push_str(&detail_lines.join(", "));
    }
    if suggests_help {
        if !message.ends_with('.') {
            message.push('.');
        }
        message.push_str(" Use `--help` for usage.");
    }

    message
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawInvocation<'a> {
    command_tokens: Vec<&'a str>,
    recognized_command: bool,
    explicit_output: Option<&'a str>,
    structured_value: bool,
}

impl<'a> RawInvocation<'a> {
    fn parse(raw_args: &'a [String]) -> Self {
        let command_tokens = raw_command_tokens(raw_args);
        let recognized_command = raw_command_tokens_are_known(&command_tokens);
        let mut explicit_output = None;
        let mut structured_value = false;
        let structured_value_mode = crate::args::CliValueMode::Structured.to_string();

        for (index, arg) in raw_args.iter().enumerate().skip(1) {
            if arg == "--" {
                break;
            }
            if recognized_command
                && arg == "--value"
                && raw_args
                    .get(index + 1)
                    .is_some_and(|value| value == structured_value_mode.as_str())
            {
                structured_value = true;
            }
            if recognized_command
                && arg
                    .strip_prefix("--value=")
                    .is_some_and(|value| value == structured_value_mode.as_str())
            {
                structured_value = true;
            }
            if let Some(value) = arg.strip_prefix("--output=") {
                explicit_output = Some(value);
            }
            if arg == "--output"
                && let Some(value) = raw_args.get(index + 1)
            {
                explicit_output = Some(value.as_str());
            }
        }

        Self {
            command_tokens,
            recognized_command,
            explicit_output,
            structured_value,
        }
    }

    fn prefers_json_errors(&self) -> bool {
        let json_output_mode = crate::args::CliOutputMode::Json.to_string();
        let text_output_mode = crate::args::CliOutputMode::Text.to_string();
        let html_output_mode = crate::args::CliOutputMode::Html.to_string();
        let none_output_mode = crate::args::CliOutputMode::None.to_string();
        let index_json_output_mode = crate::args::CliSchemaOutputMode::IndexJson.to_string();

        match self.explicit_output {
            Some(value) if value == json_output_mode.as_str() => true,
            Some(value) if value == index_json_output_mode.as_str() => true,
            Some(value)
                if value == text_output_mode.as_str()
                    || value == html_output_mode.as_str()
                    || value == none_output_mode.as_str() =>
            {
                false
            }
            _ => self.structured_value,
        }
    }

    fn command_name(&self) -> String {
        let Some(first_token) = self.command_tokens.first().copied() else {
            return TOOL_NAME.to_owned();
        };

        if let Some(contract) = crate::contract::cli_operation_catalog()
            .iter()
            .filter(|contract| self.command_tokens.len() >= contract.command_path.len())
            .find(|contract| self.command_tokens.starts_with(contract.command_path))
        {
            return contract.report_command();
        }

        match first_token {
            "catalog" => "catalog".to_owned(),
            "schema" => "schema".to_owned(),
            "inspect" => "inspect".to_owned(),
            command => command.to_owned(),
        }
    }
}

fn raw_option_tokens(raw_args: &[String]) -> impl Iterator<Item = &str> {
    raw_args
        .iter()
        .skip(1)
        .take_while(|arg| arg.as_str() != "--")
        .map(String::as_str)
}

fn raw_command_tokens(raw_args: &[String]) -> Vec<&str> {
    let tokens = raw_args
        .iter()
        .skip(1)
        .take_while(|arg| arg.as_str() != "--")
        .map(String::as_str)
        .collect::<Vec<_>>();
    let Some(command_start) = first_command_token_index(&tokens) else {
        return Vec::new();
    };
    let remaining = &tokens[command_start..];

    longest_known_command_path(remaining).unwrap_or_else(|| vec![remaining[0]])
}

fn root_option_tokens_are_known(raw_args: &[String]) -> bool {
    raw_args
        .iter()
        .skip(1)
        .take_while(|arg| arg.as_str() != "--")
        .map(String::as_str)
        .all(root_option_token_is_known)
}

fn raw_command_tokens_are_known(command_tokens: &[&str]) -> bool {
    if command_tokens.is_empty() {
        return false;
    }

    longest_known_command_path(command_tokens).is_some()
}

fn first_command_token_index(tokens: &[&str]) -> Option<usize> {
    let mut index = 0usize;
    while let Some(token) = tokens.get(index) {
        if !token.starts_with('-') {
            return Some(index);
        }
        if !root_option_token_is_known(token) {
            return None;
        }
        index += 1;
    }

    None
}

fn longest_known_command_path<'a>(tokens: &[&'a str]) -> Option<Vec<&'a str>> {
    const KNOWN_COMMAND_PATHS: &[&[&str]] = &[
        &["inspect", "source"],
        &["inspect", "select"],
        &["inspect", "slice"],
        &["catalog"],
        &["schema"],
        &["select"],
        &["slice"],
        &["inspect"],
    ];

    KNOWN_COMMAND_PATHS
        .iter()
        .find(|path| tokens.starts_with(path))
        .map(|path| path.to_vec())
}

fn root_option_token_is_known(arg: &str) -> bool {
    matches!(arg, "--help" | "--quiet" | "--verbose" | "--version")
        || short_flag_cluster_is_known(arg)
}

fn short_flag_cluster_is_known(arg: &str) -> bool {
    let flags = &arg[1..];
    if flags.is_empty() || arg.starts_with("--") {
        return false;
    }

    flags
        .chars()
        .all(|flag| matches!(flag, 'v' | 'q' | 'V' | 'h'))
}

fn token_requests_help(arg: &str) -> bool {
    arg == "--help" || short_flag_cluster_contains(arg, 'h')
}

fn token_requests_version(arg: &str) -> bool {
    arg == "--version" || short_flag_cluster_contains(arg, 'V')
}

fn short_flag_cluster_contains(arg: &str, needle: char) -> bool {
    let flags = &arg[1..];
    if flags.is_empty() || arg.starts_with("--") {
        return false;
    }

    flags.chars().any(|flag| flag == needle)
}
