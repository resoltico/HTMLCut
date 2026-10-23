// SPDX-License-Identifier: MPL-2.0
//! Repository-only literal fixture discovery and oracle validation.
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

// Facts stay open-ended; only object shape and duplicate top-level keys are checked.
struct ErrorOracle(Value);
impl<'de> Deserialize<'de> for ErrorOracle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Facts;
        impl<'de> serde::de::Visitor<'de> for Facts {
            type Value = ErrorOracle;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an error-facts object with unique keys")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut facts = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    if facts.insert(key.clone(), value).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate error-fact key: {key}"
                        )));
                    }
                }
                Ok(ErrorOracle(Value::Object(facts)))
            }
        }
        deserializer.deserialize_map(Facts)
    }
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    #[serde(default, deserialize_with = "present")]
    pub encoding: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub base_url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub max_input_bytes: Option<usize>,
    #[serde(default, deserialize_with = "present")]
    pub max_nodes: Option<usize>,
    #[serde(default, deserialize_with = "present")]
    pub max_reading_bytes: Option<usize>,
    #[serde(default, deserialize_with = "present")]
    pub max_output_bytes: Option<usize>,
    #[serde(default)]
    pub raw: bool,
    #[serde(default)]
    pub partial_error_facts: bool,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outline {
    #[serde(default, deserialize_with = "present")]
    pub select: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub depth: Option<usize>,
    #[serde(default, deserialize_with = "present")]
    pub max_lines: Option<usize>,
    #[serde(default)]
    pub attrs: bool,
}
pub struct Case {
    identity: String,
    pub path: PathBuf,
    // The CLI acquires this validated file by path; core/runtime consume its bytes.
    #[allow(dead_code)]
    pub page: Vec<u8>,
    pub options: Options,
    pub contract: Option<String>,
    pub outline: Option<Outline>,
    pub expected: Option<Value>,
    pub error: Option<Value>,
    pub bytes: Vec<u8>,
}
fn regular(path: &Path, directory: bool) {
    let meta = fs::symlink_metadata(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(
        !meta.file_type().is_symlink(),
        "redirected fixture: {path:?}"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            meta.file_attributes() & 0x400,
            0,
            "reparse fixture: {path:?}"
        );
    }
    assert!(
        if directory {
            meta.is_dir()
        } else {
            meta.is_file()
        },
        "unexpected fixture type: {path:?}"
    );
}
fn optional(path: &Path) -> Option<Vec<u8>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => panic!("{path:?}: {e}"),
        Ok(_) => {
            regular(path, false);
            Some(fs::read(path).unwrap())
        }
    }
}
/// Discover every case beneath the contract and outline groups.
pub fn discover(root: &Path) -> Vec<Case> {
    fn visit(path: &Path, group: &Path, kind: &str, result: &mut Vec<Case>) {
        regular(path, true);
        if path.join("page.html").exists() {
            let mut case = load(path, kind);
            case.identity = format!(
                "{kind}/{}",
                path.strip_prefix(group)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            result.push(case);
            return;
        }
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        assert!(!entries.is_empty(), "empty area: {path:?}");
        for entry in entries {
            visit(&entry, group, kind, result);
        }
    }
    regular(root, true);
    let mut result = Vec::new();
    for kind in ["contract", "outline"] {
        let group = root.join(kind);
        visit(&group, &group, kind, &mut result);
    }
    assert_eq!(result.len(), page_count(root), "discovery skipped a case");
    result
}
/// Independent page-file count, guarding against a filtered discovery result.
pub fn page_count(root: &Path) -> usize {
    fs::read_dir(root)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let kind = e.file_type().unwrap();
            if kind.is_dir() {
                page_count(&e.path())
            } else {
                usize::from(e.file_name() == "page.html")
            }
        })
        .sum()
}
pub fn load(path: &Path, kind: &str) -> Case {
    regular(path, true);
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap().path();
        regular(&entry, false);
        let name = entry.file_name().unwrap().to_str().unwrap();
        assert!(
            matches!(
                name,
                "page.html" | "options.json" | "error.json" | "README.md" | "PROVENANCE.md"
            ) || (kind == "contract"
                && matches!(name, "contract.json" | "expected.json" | "expected.raw"))
                || (kind == "outline" && matches!(name, "outline.json" | "expected.txt")),
            "unexpected fixture entry: {entry:?}"
        );
    }
    let read =
        |name| optional(&path.join(name)).unwrap_or_else(|| panic!("missing {name}: {path:?}"));
    let options: Options = optional(&path.join("options.json"))
        .map(|v| serde_json::from_slice(&v).unwrap())
        .unwrap_or_default();
    let error: Option<Value> = optional(&path.join("error.json"))
        .map(|v| serde_json::from_slice::<ErrorOracle>(&v).unwrap().0);
    if let Some(error) = &error {
        assert!(
            error.as_object().is_some_and(|o| o
                .get("error")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())),
            "missing error discriminant: {path:?}"
        );
    }
    assert!(
        !options.partial_error_facts || error.is_some(),
        "irrelevant partial facts: {path:?}"
    );
    assert!(kind == "contract" || !options.raw, "raw outline: {path:?}");
    let json = optional(&path.join("expected.json"));
    let raw = optional(&path.join("expected.raw"));
    let text = optional(&path.join("expected.txt"));
    let expected = json.as_ref().map(|v| {
        let value: Value = serde_json::from_slice(v).unwrap();
        let mut canonical = serde_json::to_vec(&value).unwrap();
        canonical.push(b'\n');
        assert_eq!(*v, canonical, "noncanonical JSON oracle: {path:?}");
        value
    });
    let bytes = if error.is_some() {
        assert!(
            json.is_none() && raw.is_none() && text.is_none(),
            "conflicting success/error oracles: {path:?}"
        );
        Vec::new()
    } else if kind == "contract" {
        assert!(
            json.is_some() && text.is_none(),
            "missing/irrelevant JSON oracle: {path:?}"
        );
        assert_eq!(
            raw.is_some(),
            options.raw,
            "missing/irrelevant raw oracle: {path:?}"
        );
        if options.raw {
            raw.unwrap()
        } else {
            json.unwrap()
        }
    } else {
        assert!(
            text.is_some() && json.is_none() && raw.is_none(),
            "missing/irrelevant outline oracle: {path:?}"
        );
        text.unwrap()
    };
    Case {
        identity: format!("{kind}/{}", path.file_name().unwrap().to_string_lossy()),
        path: path.to_owned(),
        page: read("page.html"),
        options,
        contract: (kind == "contract").then(|| String::from_utf8(read("contract.json")).unwrap()),
        outline: (kind == "outline")
            .then(|| serde_json::from_slice(&read("outline.json")).unwrap()),
        expected,
        error,
        bytes,
    }
}
impl Case {
    pub fn identity(&self) -> String {
        self.identity.clone()
    }
    pub fn assert_error(&self, mut actual: Value) {
        actual.as_object_mut().unwrap().remove("message");
        let mut expected = self.error.clone().unwrap();
        expected.as_object_mut().unwrap().remove("message");
        if self.options.partial_error_facts {
            for (key, value) in expected.as_object().unwrap() {
                assert_eq!(actual.get(key), Some(value), "{:?}: {key}", self.path);
            }
        } else {
            assert_eq!(actual, expected, "{:?}", self.path);
        }
    }
    pub fn assert_bytes(&self, actual: &[u8]) {
        assert_eq!(actual, self.bytes, "{:?}", self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_error_discriminants_and_facts_refuse_without_touching_contract_text() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("page.html"), b"<p>x</p>").unwrap();
        fs::write(root.path().join("contract.json"), b"\"aside\"").unwrap();
        for partial in [false, true] {
            fs::write(
                root.path().join("options.json"),
                format!(r#"{{"partial_error_facts":{partial}}}"#),
            )
            .unwrap();
            // Last values match the ordinary absent-selection refusal; discarding
            // duplicates must not let either exact or partial comparison accept it.
            for oracle in [
                br#"{"error":"invalid_contract","error":"count_mismatch","path":"$","select":"aside","found":0,"expected":{"min":1,"max":1}}"#.as_slice(),
                br#"{"error":"count_mismatch","path":"$","select":"aside","found":1,"found":0,"expected":{"min":1,"max":1}}"#.as_slice(),
            ] {
                fs::write(root.path().join("error.json"), oracle).unwrap();
                assert!(std::panic::catch_unwind(|| load(root.path(), "contract")).is_err());
            }
        }
        let contract = br#"{"select":"p","select":"aside"}"#;
        fs::write(root.path().join("contract.json"), contract).unwrap();
        fs::write(
            root.path().join("error.json"),
            br#"{"error":"invalid_contract"}"#,
        )
        .unwrap();
        assert_eq!(
            load(root.path(), "contract").contract.unwrap().as_bytes(),
            contract
        );
    }
    #[test]
    fn rejects_invalid_metadata_and_oracle_shapes_without_git() {
        for options in [
            r#"{"max_nods":1}"#,
            r#"{"max_nodes":"1"}"#,
            r#"{"max_nodes":null}"#,
            r#"{"max_nodes":1,"max_nodes":2}"#,
            r#"{"max_nodes":-1}"#,
            r#"{"max_nodes":18446744073709551616}"#,
        ] {
            assert!(
                serde_json::from_str::<Options>(options).is_err(),
                "{options}"
            );
        }
        let dir = tempfile::tempdir().unwrap();
        let case = dir.path();
        fs::write(case.join("page.html"), b"<p>x</p>").unwrap();
        fs::write(case.join("contract.json"), b"\"p\"").unwrap();
        assert!(std::panic::catch_unwind(|| load(case, "contract")).is_err());
        fs::write(case.join("expected.json"), b"\"x\"\n").unwrap();
        load(case, "contract").assert_bytes(b"\"x\"\n");
        fs::write(case.join("options.json"), b"{\"raw\":true}").unwrap();
        assert!(std::panic::catch_unwind(|| load(case, "contract")).is_err());
        fs::write(case.join("expected.raw"), b"x\n").unwrap();
        load(case, "contract");
        fs::write(case.join("error.json"), b"{\"error\":\"count_mismatch\"}").unwrap();
        assert!(std::panic::catch_unwind(|| load(case, "contract")).is_err());
        for file in ["expected.json", "expected.raw"] {
            fs::remove_file(case.join(file)).unwrap();
        }
        fs::write(case.join("error.json"), b"{}").unwrap();
        assert!(std::panic::catch_unwind(|| load(case, "contract")).is_err());
        // Empty outline is rejected even when extraction discovery is nonempty.
        let root = tempfile::tempdir().unwrap();
        for kind in ["contract", "outline"] {
            fs::create_dir(root.path().join(kind)).unwrap();
        }
        let valid = root.path().join("contract/valid");
        fs::create_dir(&valid).unwrap();
        fs::write(valid.join("page.html"), b"<p>x</p>").unwrap();
        fs::write(valid.join("contract.json"), b"\"p\"").unwrap();
        fs::write(valid.join("expected.json"), b"\"x\"\n").unwrap();
        assert!(std::panic::catch_unwind(|| discover(root.path())).is_err());
        let outline = root.path().join("outline/valid");
        fs::create_dir(&outline).unwrap();
        fs::write(outline.join("page.html"), b"<p>x</p>").unwrap();
        fs::write(outline.join("outline.json"), b"{}").unwrap();
        fs::write(outline.join("expected.txt"), b"literal").unwrap();
        assert_eq!(discover(root.path()).len(), 2);
        fs::remove_file(valid.join("expected.json")).unwrap();
        fs::write(
            valid.join("options.json"),
            b"{\"partial_error_facts\":true}",
        )
        .unwrap();
        fs::write(valid.join("error.json"), b"{\"error\":\"count_mismatch\"}").unwrap();
        let partial = load(&valid, "contract");
        assert!(
            std::panic::catch_unwind(
                || partial.assert_error(serde_json::json!({"error":"invalid_contract"}))
            )
            .is_err()
        );
        #[cfg(unix)]
        {
            fs::remove_file(valid.join("options.json")).unwrap();
            std::os::unix::fs::symlink(outline.join("outline.json"), valid.join("options.json"))
                .unwrap();
            assert!(std::panic::catch_unwind(|| load(&valid, "contract")).is_err());
        }
        let root = tempfile::tempdir().unwrap();
        for kind in ["contract", "outline"] {
            fs::create_dir(root.path().join(kind)).unwrap();
        }
        assert!(std::panic::catch_unwind(|| discover(root.path())).is_err());
        fs::write(root.path().join("contract/stray"), b"x").unwrap();
        assert!(std::panic::catch_unwind(|| discover(root.path())).is_err());
    }
}
