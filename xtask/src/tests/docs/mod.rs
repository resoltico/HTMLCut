use super::*;

fn write_minimal_docs_legal_scaffold(repo_root: &Path, version: &str, updated: &str) {
    fs::write(
        repo_root.join("deny.toml"),
        r#"[licenses]
allow = [
    "MIT",
]
"#,
    )
    .expect("write deny.toml");
    fs::write(
        repo_root.join("PATENTS.md"),
        format!(
            "<!--\nAFAD:\n  afad: \"3.5\"\n  version: \"{version}\"\n  domain: LEGAL\n  updated: \"{updated}\"\nRETRIEVAL_HINTS:\n  keywords: [patents]\n  questions: [\"q\"]\n-->\n\n# Patent Notes\n\n| License family | Explicit patent grant | Notes |\n|:---------------|:----------------------|:------|\n| MIT | No explicit grant | HTMLCut itself is MIT-licensed. |\n"
        ),
    )
    .expect("write patents");
}

mod commands;
mod contracts;
mod legal;
mod metadata;
mod paths;
mod release;
