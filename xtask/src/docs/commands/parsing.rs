pub(crate) fn extract_htmlcut_examples(text: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut in_fence = false;
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            let was_in_fence = in_fence;
            in_fence = match was_in_fence {
                false => true,
                true => false,
            };
            if was_in_fence {
                current.clear();
            }
            continue;
        }

        if in_fence {
            if current.is_empty()
                && (!trimmed.starts_with("htmlcut ") || !is_concrete_example(trimmed))
            {
                continue;
            }

            if current.is_empty() {
                current.push_str(trimmed.trim_end_matches('\\').trim_end());
            } else {
                current.push(' ');
                current.push_str(trimmed.trim_end_matches('\\').trim_end());
            }

            if !trimmed.ends_with('\\') {
                examples.push(std::mem::take(&mut current));
            }
        }
    }

    examples
}

fn is_concrete_example(line: &str) -> bool {
    !line.contains('[') && !line.contains("...")
}

pub(crate) fn shell_words(command: &str) -> Result<Vec<String>, String> {
    shell_words::split(command).map_err(|error| error.to_string())
}

pub(super) fn option_value<'a>(tokens: &'a [String], flag: &str) -> Option<&'a str> {
    tokens.iter().enumerate().find_map(|(index, token)| {
        token.strip_prefix(&format!("{flag}=")).or_else(|| {
            if token == flag {
                tokens.get(index + 1).map(String::as_str)
            } else {
                None
            }
        })
    })
}

pub(crate) fn command_path(tokens: &[String]) -> Vec<&str> {
    match tokens.get(1).map(String::as_str) {
        Some("inspect") => tokens.get(2).map_or_else(
            || vec!["inspect"],
            |subcommand| vec!["inspect", subcommand.as_str()],
        ),
        Some(top_level) => vec![top_level],
        None => Vec::new(),
    }
}

pub(super) fn clap_error_message(error: &clap::Error) -> String {
    let rendered = error.to_string();
    rendered
        .lines()
        .find_map(|line| line.strip_prefix("error: ").map(ToOwned::to_owned))
        .unwrap_or_else(|| rendered.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_are_collected_only_from_closed_concrete_htmlcut_fences() {
        let text = "htmlcut select outside.html\n```bash\nhtmlcut select inside.html\n```\n```bash\nhtmlcut select [placeholder]\n```\n";

        assert_eq!(
            extract_htmlcut_examples(text),
            vec!["htmlcut select inside.html".to_owned()]
        );
    }
}
