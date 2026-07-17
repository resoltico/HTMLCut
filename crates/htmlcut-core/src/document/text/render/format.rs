use crate::contracts::WhitespaceMode;

pub(crate) fn collapse_inline_whitespace(input: &str) -> String {
    let mut output = String::new();
    let mut previous_was_whitespace = false;

    for character in input.chars() {
        if character.is_whitespace() {
            previous_was_whitespace = true;
            continue;
        }

        if previous_was_whitespace && !output.is_empty() {
            output.push(' ');
        }

        output.push(character);
        previous_was_whitespace = false;
    }

    output
}

pub(crate) fn needs_space(output: &str, next_text: &str) -> bool {
    let Some(last_character) = output.chars().next_back() else {
        return false;
    };
    let Some(first_character) = next_text.chars().next() else {
        return false;
    };

    !last_character.is_whitespace()
        && !matches!(last_character, '(' | '[' | '{' | '/' | '-')
        && !matches!(
            first_character,
            ')' | ']' | '}' | ',' | '.' | ';' | ':' | '!' | '?'
        )
}

pub(crate) fn push_newline(output: &mut String, count: usize) {
    let trimmed_len = output.trim_end_matches('\n').len();
    output.truncate(trimmed_len);
    if !output.is_empty() {
        output.push_str(&"\n".repeat(count));
    }
}

pub(crate) fn apply_whitespace_mode(input: &str, whitespace: WhitespaceMode) -> String {
    match whitespace {
        WhitespaceMode::Rendered => input.trim_matches('\n').to_owned(),
        WhitespaceMode::Normalize => {
            let mut lines = Vec::new();
            let mut blank_streak = 0usize;

            for line in input.lines() {
                let trimmed = normalize_structured_line(line);
                if trimmed.is_empty() {
                    blank_streak += 1;
                    lines.extend((blank_streak == 1).then_some(String::new()));
                } else {
                    blank_streak = 0;
                    lines.push(trimmed);
                }
            }

            lines.join("\n").trim_matches('\n').to_owned()
        }
    }
}

pub(super) fn normalize_rendered_output(output: String, whitespace: WhitespaceMode) -> String {
    let normalized = remove_immediate_heading_echoes(&collapse_blank_lines(
        &output
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n"),
    ));

    apply_whitespace_mode(normalized.trim_matches('\n'), whitespace)
}

pub(super) fn normalize_heading_text(rendered: &str) -> String {
    collapse_inline_whitespace(rendered.trim())
}

pub(super) fn normalize_structured_line(line: &str) -> String {
    let trimmed_start = line.trim_start();
    let indent = &line[..line.len() - trimmed_start.len()];
    let collapsed = collapse_inline_whitespace(trimmed_start);
    if collapsed.is_empty() {
        String::new()
    } else {
        format!("{indent}{collapsed}")
    }
}

pub(super) fn push_prefixed_block(output: &mut String, block: &str, prefix: &str) {
    if block.is_empty() {
        return;
    }

    let normalized = collapse_blank_lines(block);
    for (index, line) in normalized.lines().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        output.push_str(prefix);
        if !line.is_empty() {
            output.push_str(line);
        }
    }
}

pub(super) fn collapse_blank_lines(input: &str) -> String {
    let mut collapsed = String::with_capacity(input.len());
    let mut consecutive_newlines = 0usize;

    for ch in input.chars() {
        if ch == '\n' {
            if consecutive_newlines < 2 {
                collapsed.push(ch);
            }
            consecutive_newlines += 1;
        } else {
            consecutive_newlines = 0;
            collapsed.push(ch);
        }
    }

    collapsed
}

pub(super) fn remove_immediate_heading_echoes(input: &str) -> String {
    let lines = input.lines().collect::<Vec<_>>();
    let mut output = Vec::<String>::new();
    let mut index = 0usize;

    while index < lines.len() {
        let current = lines[index];
        output.push(current.to_owned());

        if let Some(heading_text) = current
            .strip_prefix('#')
            .map(|_| current.trim_start_matches('#').trim())
            .filter(|heading_text| !heading_text.is_empty())
        {
            if lines.get(index + 1) == Some(&"")
                && lines
                    .get(index + 2)
                    .is_some_and(|line| line.trim() == heading_text)
            {
                index += 3;
                index += usize::from(lines.get(index) == Some(&""));
                output.push(String::new());
                continue;
            }

            let mut duplicate_index = index + 1;
            while lines.get(duplicate_index) == Some(&"") {
                duplicate_index += 1;
            }
            if lines
                .get(duplicate_index)
                .is_some_and(|line| line.trim() == current.trim())
            {
                index = duplicate_index + 1;
                index += usize::from(lines.get(index) == Some(&""));
                output.push(String::new());
                continue;
            }
        }

        index += 1;
    }

    output.join("\n")
}

#[cfg(test)]
pub(crate) fn collapse_blank_lines_for_tests(input: &str) -> String {
    collapse_blank_lines(input)
}
