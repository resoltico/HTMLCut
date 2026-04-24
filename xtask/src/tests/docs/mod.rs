use super::*;

fn write_docs_protocol(repo_root: &Path, afad_version: &str) {
    let codex_dir = repo_root.join(".codex");
    fs::create_dir_all(&codex_dir).expect("create .codex");
    fs::write(
        codex_dir.join("PROTOCOL_AFAD.md"),
        format!(
            "# PROTOCOL_AFAD.md — Agent-First Documentation Protocol\n\nProtocol: `AGENT_FIRST_DOCUMENTATION`\nVersion: `{afad_version}`\n"
        ),
    )
    .expect("write protocol");
}

fn write_minimal_docs_legal_scaffold(repo_root: &Path, version: &str, updated: &str) {
    write_docs_protocol(repo_root, "4.0");
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
            "<!--\nAFAD:\n  afad: \"4.0\"\n  version: \"{version}\"\n  domain: LEGAL\n  updated: \"{updated}\"\nRETRIEVAL_HINTS:\n  keywords: [patents]\n  questions: [\"q\"]\n-->\n\n# Patent Notes\n\n| License family | Explicit patent grant | Notes |\n|:---------------|:----------------------|:------|\n| MIT | No explicit grant | HTMLCut itself is MIT-licensed. |\n"
        ),
    )
    .expect("write patents");
}

mod commands;
mod contracts;
mod inventory;
mod legal;
mod metadata;
mod paths;
mod release;
