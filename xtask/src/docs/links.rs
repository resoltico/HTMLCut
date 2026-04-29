use std::path::Path;

use regex::Regex;

use super::paths::repo_relative_display;

fn is_absolute_local_link_target(target: &str) -> bool {
    if target.starts_with('/') || target.starts_with('\\') {
        return true;
    }

    let bytes = target.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

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
            .trim();
        let target = target.strip_prefix('<').unwrap_or(target);
        let target = target.strip_suffix('>').unwrap_or(target);
        let target = target.split('#').next().unwrap_or_default();
        if target.is_empty()
            || target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
        {
            continue;
        }

        if is_absolute_local_link_target(target) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_tempdir::tempdir;
    use std::fs;

    #[test]
    fn local_link_errors_trim_wrapped_targets_and_recognize_absolute_forms() {
        let repo_root = tempdir().expect("tempdir");
        let docs_dir = repo_root.path().join("docs");
        fs::create_dir_all(&docs_dir).expect("create docs dir");
        let doc_path = docs_dir.join("guide.md");
        fs::write(&doc_path, "# Guide\n").expect("write guide");
        let link_pattern = Regex::new(r"\[[^\]]+\]\(([^)]+)\)").expect("compile link pattern");
        let errors = local_link_errors(
            repo_root.path(),
            &doc_path,
            "[wrapped-local](<./missing.md>)\n[posix](/tmp/nope)\n[windows](C:\\\\nope)\n[unc](\\\\server\\share)\n",
            &link_pattern,
        );

        assert!(
            errors
                .iter()
                .any(|error| error.contains("links to missing local path: ./missing.md"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("absolute local link target: /tmp/nope"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains(r"absolute local link target: C:\\nope"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains(r"absolute local link target: \\server\share"))
        );
    }

    #[test]
    fn local_link_errors_cover_short_relative_targets() {
        let repo_root = tempdir().expect("tempdir");
        let docs_dir = repo_root.path().join("docs");
        fs::create_dir_all(&docs_dir).expect("create docs dir");
        let doc_path = docs_dir.join("guide.md");
        fs::write(&doc_path, "# Guide\n").expect("write guide");
        fs::write(docs_dir.join("a"), "ok\n").expect("write short relative target");
        let link_pattern = Regex::new(r"\[[^\]]+\]\(([^)]+)\)").expect("compile link pattern");
        let errors = local_link_errors(repo_root.path(), &doc_path, "[short](a)\n", &link_pattern);

        assert!(errors.is_empty(), "unexpected link errors: {errors:#?}");
    }
}
