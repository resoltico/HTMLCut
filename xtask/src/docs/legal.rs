use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::model::DynResult;

pub(super) fn legal_doc_errors(repo_root: &Path, display_path: &str, text: &str) -> Vec<String> {
    if display_path != "PATENTS.md" {
        return Vec::new();
    }

    let policy_path = repo_root.join("deny.toml");
    let allowed_license_families = match allowed_license_families(&policy_path) {
        Ok(families) => families,
        Err(error) => {
            return vec![format!(
                "{display_path} could not load canonical license allowlist from {}: {error}",
                policy_path.display()
            )];
        }
    };
    let documented_license_families = documented_license_families(text);

    let mut errors = Vec::new();

    for license_family in allowed_license_families.difference(&documented_license_families) {
        errors.push(format!(
            "{display_path} is missing allowed license family from deny.toml: {license_family}"
        ));
    }

    for license_family in documented_license_families.difference(&allowed_license_families) {
        errors.push(format!(
            "{display_path} documents a license family not allowed by deny.toml: {license_family}"
        ));
    }

    errors
}

fn allowed_license_families(policy_path: &Path) -> DynResult<BTreeSet<String>> {
    let policy = fs::read_to_string(policy_path)?;
    let mut in_licenses_section = false;
    let mut in_allow_list = false;
    let mut allowed = BTreeSet::new();

    for raw_line in policy.lines() {
        let line = raw_line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            in_licenses_section = line == "[licenses]";
            in_allow_list = false;
            continue;
        }

        if !in_licenses_section {
            continue;
        }

        if !in_allow_list {
            if let Some(rest) = line.strip_prefix("allow = [") {
                in_allow_list = true;
                collect_quoted_values(rest, &mut allowed);
                if rest.contains(']') {
                    break;
                }
            }
            continue;
        }

        collect_quoted_values(line, &mut allowed);
        if line.contains(']') {
            break;
        }
    }

    Ok(allowed)
}

fn collect_quoted_values(line: &str, values: &mut BTreeSet<String>) {
    let mut cursor = line;
    while let Some(start) = cursor.find('"') {
        let after_start = &cursor[start + 1..];
        let Some(end) = after_start.find('"') else {
            break;
        };
        values.insert(after_start[..end].to_owned());
        cursor = &after_start[end + 1..];
    }
}

fn documented_license_families(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') {
                return None;
            }

            let mut columns = trimmed.split('|').skip(1);
            let first_column = columns.next()?.trim().trim_matches('`');
            if first_column.is_empty()
                || first_column == "License family"
                || first_column
                    .chars()
                    .all(|character| character == ':' || character == '-')
            {
                return None;
            }

            Some(first_column.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_doc_errors_report_missing_policy_files_and_extra_documented_families() {
        let repo_root = htmlcut_tempdir::tempdir().expect("tempdir");

        let missing_policy_errors = legal_doc_errors(
            repo_root.path(),
            "PATENTS.md",
            "| License family | Explicit patent grant | Notes |\n| --- | --- | --- |\n| MIT | No explicit grant | note |\n",
        );
        assert_eq!(missing_policy_errors.len(), 1);
        assert!(
            missing_policy_errors[0]
                .contains("PATENTS.md could not load canonical license allowlist from")
        );

        let policy_path = repo_root.path().join("deny.toml");
        fs::write(
            &policy_path,
            r#"
[licenses]
confidence-threshold = 0.93
allow = [
    "MIT",
]
"#,
        )
        .expect("write deny.toml");

        let extra_family_errors = legal_doc_errors(
            repo_root.path(),
            "PATENTS.md",
            r#"
| License family | Explicit patent grant | Notes |
|:---------------|:----------------------|:------|
| MIT | No explicit grant | HTMLCut itself is MIT-licensed. |
| Apache-2.0 | Yes | Extra family for drift coverage. |
"#,
        );
        assert_eq!(
            extra_family_errors,
            vec![
                "PATENTS.md documents a license family not allowed by deny.toml: Apache-2.0"
                    .to_owned()
            ]
        );
    }

    #[test]
    fn allowed_license_families_reads_the_deny_allowlist() {
        let repo_root = htmlcut_tempdir::tempdir().expect("tempdir");
        let policy_path = repo_root.path().join("deny.toml");
        fs::write(
            &policy_path,
            r#"
[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "NCSA",
]
"#,
        )
        .expect("write deny.toml");

        let families = allowed_license_families(&policy_path).expect("allowlist");

        assert_eq!(
            families,
            BTreeSet::from(["Apache-2.0".to_owned(), "MIT".to_owned(), "NCSA".to_owned(),])
        );
    }

    #[test]
    fn allowed_license_families_support_inline_lists_and_ignore_non_allow_entries() {
        let repo_root = htmlcut_tempdir::tempdir().expect("tempdir");
        let policy_path = repo_root.path().join("deny.toml");
        fs::write(
            &policy_path,
            r#"
[licenses]
confidence-threshold = 0.93
allow = ["MIT"]
"#,
        )
        .expect("write deny.toml");

        let families = allowed_license_families(&policy_path).expect("allowlist");

        assert_eq!(families, BTreeSet::from(["MIT".to_owned()]));
    }

    #[test]
    fn allowed_license_families_ignore_malformed_section_headers() {
        let repo_root = htmlcut_tempdir::tempdir().expect("tempdir");
        let policy_path = repo_root.path().join("deny.toml");
        fs::write(
            &policy_path,
            r#"
[licenses
allow = ["MIT"]
[licenses]
allow = ["Apache-2.0"]
"#,
        )
        .expect("write deny.toml");

        let families = allowed_license_families(&policy_path).expect("allowlist");

        assert_eq!(families, BTreeSet::from(["Apache-2.0".to_owned()]));
    }

    #[test]
    fn collect_quoted_values_ignores_unterminated_entries() {
        let mut values = BTreeSet::new();

        collect_quoted_values(r#""MIT", "Apache-2.0"#, &mut values);

        assert_eq!(values, BTreeSet::from(["MIT".to_owned()]));
    }

    #[test]
    fn documented_license_families_reads_exact_table_rows() {
        let families = documented_license_families(
            r#"
| License family | Explicit patent grant | Notes |
|:---------------|:----------------------|:------|
| MIT | No explicit grant | HTMLCut itself is MIT-licensed. |
| NCSA | No explicit grant | Broad permissive grant, no standalone patent clause. |
| Unlicense | No explicit grant | Public-domain-like terms. |
"#,
        );

        assert_eq!(
            families,
            BTreeSet::from(["MIT".to_owned(), "NCSA".to_owned(), "Unlicense".to_owned(),])
        );
    }

    #[test]
    fn documented_license_families_ignore_non_rows_and_empty_first_columns() {
        let families = documented_license_families(
            r#"
not a table row
| License family | Explicit patent grant | Notes |
| --- | --- | --- |
| | No explicit grant | empty family |
| MIT | No explicit grant | HTMLCut itself is MIT-licensed. |
"#,
        );

        assert_eq!(families, BTreeSet::from(["MIT".to_owned()]));
    }
}
