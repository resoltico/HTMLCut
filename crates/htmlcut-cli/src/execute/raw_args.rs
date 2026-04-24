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
    rendered
        .lines()
        .find_map(|line| line.strip_prefix("error: ").map(ToOwned::to_owned))
        .unwrap_or_else(|| rendered.trim().to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawInvocation<'a> {
    command_tokens: Vec<&'a str>,
    explicit_output: Option<&'a str>,
    structured_value: bool,
}

impl<'a> RawInvocation<'a> {
    fn parse(raw_args: &'a [String]) -> Self {
        let mut explicit_output = None;
        let mut structured_value = false;
        let structured_value_mode = crate::args::CliValueMode::Structured.to_string();

        for (index, arg) in raw_args.iter().enumerate().skip(1) {
            if arg == "--" {
                break;
            }
            if arg == "--value"
                && raw_args
                    .get(index + 1)
                    .is_some_and(|value| value == structured_value_mode.as_str())
            {
                structured_value = true;
            }
            if arg
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
            command_tokens: raw_command_tokens(raw_args),
            explicit_output,
            structured_value,
        }
    }

    fn prefers_json_errors(&self) -> bool {
        let json_output_mode = crate::args::CliOutputMode::Json.to_string();
        let text_output_mode = crate::args::CliOutputMode::Text.to_string();
        let html_output_mode = crate::args::CliOutputMode::Html.to_string();
        let none_output_mode = crate::args::CliOutputMode::None.to_string();

        match self.explicit_output {
            Some(value) if value == json_output_mode.as_str() => true,
            Some(value)
                if value == text_output_mode.as_str()
                    || value == html_output_mode.as_str()
                    || value == none_output_mode.as_str() =>
            {
                false
            }
            _ => self.inspect_mode() || self.structured_value,
        }
    }

    fn command_name(&self) -> String {
        let Some(first_token) = self.command_tokens.first().copied() else {
            return TOOL_NAME.to_owned();
        };

        if let Some(contract) = htmlcut_core::cli_contract::cli_operation_catalog()
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

    fn inspect_mode(&self) -> bool {
        matches!(self.command_tokens.first(), Some(&"inspect"))
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
    raw_args
        .iter()
        .skip(1)
        .take_while(|arg| arg.as_str() != "--")
        .map(String::as_str)
        .skip_while(|arg| arg.starts_with('-'))
        .take_while(|arg| !arg.starts_with('-'))
        .collect()
}

fn root_option_tokens_are_known(raw_args: &[String]) -> bool {
    raw_args
        .iter()
        .skip(1)
        .take_while(|arg| arg.as_str() != "--")
        .take_while(|arg| arg.starts_with('-'))
        .all(|arg| {
            matches!(arg.as_str(), "--help" | "--quiet" | "--version")
                || short_flag_cluster_is_known(arg)
        })
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
