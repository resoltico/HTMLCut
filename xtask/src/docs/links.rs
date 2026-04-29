use std::path::Path;

use regex::Regex;

use super::paths::repo_relative_display;

pub(super) fn local_link_errors(
    repo_root: &Path,
    doc_path: &Path,
    text: &str,
    link_pattern: &Regex,
) -> Vec<String> {
    let display_path = repo_relative_display(repo_root, doc_path);
    let mut errors = Vec::new();

    for capture in link_pattern.captures_iter(text) {
        let target = capture
            .get(1)
            .map_or("", |target_match| target_match.as_str())
            .trim()
            .trim_matches(|character| character == '<' || character == '>');
        let target = target.split('#').next().unwrap_or_default();
        if target.is_empty()
            || target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
        {
            continue;
        }

        if Path::new(target).is_absolute() {
            errors.push(format!(
                "{display_path} uses an absolute local link target: {target}"
            ));
            continue;
        }

        let resolved = doc_path.parent().unwrap_or(Path::new("")).join(target);
        if !resolved.exists() {
            errors.push(format!(
                "{display_path} links to missing local path: {target}"
            ));
        }
    }

    errors
}
