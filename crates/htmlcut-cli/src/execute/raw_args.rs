use crate::metadata::TOOL_NAME;

pub(crate) fn raw_args_prefers_json(raw_args: &[String]) -> bool {
    let mut explicit_output = None;
    let mut inspect_mode = false;
    let mut structured_value = false;
    let structured_value_mode = crate::args::CliValueMode::Structured.to_string();
    let json_output_mode = crate::args::CliOutputMode::Json.to_string();
    let text_output_mode = crate::args::CliOutputMode::Text.to_string();
    let html_output_mode = crate::args::CliOutputMode::Html.to_string();
    let none_output_mode = crate::args::CliOutputMode::None.to_string();

    for (index, arg) in raw_args.iter().enumerate().skip(1) {
        if arg == "inspect" {
            inspect_mode = true;
        }
        if arg == "--value"
            && raw_args
                .get(index + 1)
                .is_some_and(|value| value == structured_value_mode.as_str())
        {
            structured_value = true;
        }
        if let Some(value) = arg.strip_prefix("--output=") {
            explicit_output = Some(value.to_owned());
        }
        if arg == "--output"
            && let Some(value) = raw_args.get(index + 1)
        {
            explicit_output = Some(value.clone());
        }
    }

    match explicit_output.as_deref() {
        Some(value) if value == json_output_mode.as_str() => true,
        Some(value)
            if value == text_output_mode.as_str()
                || value == html_output_mode.as_str()
                || value == none_output_mode.as_str() =>
        {
            false
        }
        _ => inspect_mode || structured_value,
    }
}

pub(crate) fn raw_args_requests_version(raw_args: &[String]) -> bool {
    raw_option_tokens(raw_args).any(|arg| matches!(arg, "--version" | "-V"))
}

pub(crate) fn raw_args_requests_help(raw_args: &[String]) -> bool {
    raw_option_tokens(raw_args).any(|arg| matches!(arg, "--help" | "-h"))
}

pub(crate) fn command_name_from_raw_args(raw_args: &[String]) -> String {
    let command_tokens = raw_command_tokens(raw_args);
    let Some(first_token) = command_tokens.first().copied() else {
        return TOOL_NAME.to_owned();
    };

    if let Some(contract) = htmlcut_core::cli_operation_catalog()
        .iter()
        .filter(|contract| command_tokens.len() >= contract.command_path.len())
        .find(|contract| command_tokens.starts_with(contract.command_path))
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

pub(crate) fn clap_error_message(error: &clap::Error) -> String {
    let rendered = error.to_string();
    rendered
        .lines()
        .find_map(|line| line.strip_prefix("error: ").map(ToOwned::to_owned))
        .unwrap_or_else(|| rendered.trim().to_owned())
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
