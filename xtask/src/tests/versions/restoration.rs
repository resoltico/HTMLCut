// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn published_fork_versions_preserve_normalized_core_features() {
    let normalized = r#"[dependencies.scraper]
version = "0.27.0"
features = ["errors", "core-site"]
default-features = false
optional = true
[dependencies.selectors]
version = "0.38.0"
[dependencies.sha2]
version = "0.11.0"
features = ["std"]
"#;
    let restored =
        restore_vendored_dependency_paths_in_baseline_manifest(normalized, PUBLISHED_WORKSPACE)
            .unwrap()
            .unwrap();
    let value: toml::Value = toml::from_str(&restored).unwrap();
    for (alias, version) in [
        ("scraper", "0.27.0-htmlcut.9"),
        ("selectors", "0.41.0-htmlcut.1"),
        ("sha2", "0.11.0-htmlcut.3"),
    ] {
        assert_eq!(
            value["dependencies"][alias]["version"].as_str(),
            Some(version)
        );
        assert_eq!(
            value["dependencies"][alias]["path"].as_str(),
            Some(format!("vendor/{alias}").as_str())
        );
    }
    let scraper = &value["dependencies"]["scraper"];
    assert_eq!(
        scraper["features"].as_array().unwrap(),
        &vec![
            toml::Value::String("errors".into()),
            toml::Value::String("core-site".into())
        ]
    );
    assert_eq!(scraper["default-features"].as_bool(), Some(false));
    assert_eq!(scraper["optional"].as_bool(), Some(true));
    assert_eq!(
        value["dependencies"]["sha2"]["package"].as_str(),
        Some("htmlcut-sha2")
    );
    assert_eq!(
        value["dependencies"]["sha2"]["features"]
            .as_array()
            .unwrap(),
        &vec![toml::Value::String("std".into())]
    );
}

#[test]
fn published_declarations_skip_unrelated_or_incomplete_shapes() {
    let normalized = "[dependencies.scraper]\nversion = '0.27'\n";
    for published in [
        "[package]\nname='sample'\n",
        "[workspace]\n",
        "[workspace.dependencies]\nscraper='0.27'\n",
        "[workspace.dependencies.scraper]\npath='patches/rust/scraper'\n",
        "[workspace.dependencies.scraper]\npackage='htmlcut-scraper'\n",
        "[workspace.dependencies.scraper]\npackage='scraper'\npath='patches/rust/scraper'\n",
        "[workspace.dependencies.scraper]\npackage='htmlcut-scraper'\npath='vendor/scraper'\n",
    ] {
        assert_eq!(
            restore_vendored_dependency_paths_in_baseline_manifest(normalized, published).unwrap(),
            None
        );
    }
}

#[test]
fn published_alias_and_omitted_version_are_authoritative() {
    let normalized = "[dependencies.paint]\nversion = '0.38'\nfeatures=['core-site']\n";
    let published = "[workspace.dependencies.paint]\npackage='htmlcut-selectors'\npath='patches/rust/selectors'\n";
    let restored = restore_vendored_dependency_paths_in_baseline_manifest(normalized, published)
        .unwrap()
        .unwrap();
    let value: toml::Value = toml::from_str(&restored).unwrap();
    let dependency = &value["dependencies"]["paint"];
    assert_eq!(dependency["package"].as_str(), Some("htmlcut-selectors"));
    assert_eq!(dependency["path"].as_str(), Some("vendor/selectors"));
    assert!(dependency.get("version").is_none());
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &vec![toml::Value::String("core-site".into())]
    );
}

#[test]
fn malformed_published_workspace_rejects_before_empty_baseline_skip() {
    let error = restore_vendored_dependency_paths_in_baseline_manifest(
        "[package]\nname='sample'\n",
        "[broken",
    )
    .unwrap_err();
    assert!(error.to_string().contains("published workspace Cargo.toml"));
}
