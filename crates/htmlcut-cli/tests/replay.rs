// SPDX-License-Identifier: MPL-2.0
mod support;
use std::io::Read;
use support::invoke;

fn generated(root: &std::path::Path) -> std::path::PathBuf {
    let path = root.join("snapshot.htmlcut.tar");
    let output = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "p",
            "--bundle",
            path.to_str().unwrap(),
        ],
        b"<p>180</p>",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"180\"]\n");
    path
}
fn members(path: &std::path::Path) -> Vec<(String, Vec<u8>, tar::EntryType)> {
    let mut archive = tar::Archive::new(std::fs::File::open(path).unwrap());
    archive
        .entries()
        .unwrap()
        .raw(true)
        .map(|entry| {
            let mut entry = entry.unwrap();
            let name = String::from_utf8(entry.path_bytes().into_owned()).unwrap();
            let kind = entry.header().entry_type();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            (name, data, kind)
        })
        .collect()
}
fn store(path: &std::path::Path, entries: &[(String, Vec<u8>, tar::EntryType)]) {
    let mut archive = tar::Builder::new(std::fs::File::create(path).unwrap());
    for (name, data, kind) in entries {
        let mut header = tar::Header::new_ustar();
        header.set_mode(0o600);
        header.set_entry_type(*kind);
        header.set_size(data.len() as u64);
        let name_bytes = name.as_bytes();
        assert!(name_bytes.len() < 100);
        header.as_ustar_mut().unwrap().name[..name_bytes.len()].copy_from_slice(name_bytes);
        if *kind == tar::EntryType::Symlink {
            header.set_link_name("../outside").unwrap();
        }
        header.set_cksum();
        archive.append(&header, data.as_slice()).unwrap();
    }
    archive.finish().unwrap();
}
fn fails(path: &std::path::Path, code: i32) {
    let result = invoke(&["run", path.to_str().unwrap()], b"");
    assert_eq!(
        result.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn moved_bundle_replays_after_original_source_and_plan_are_deleted() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let source = root.path().join("source.html");
    let plan = root.path().join("plan.json");
    let bundle = root.path().join("source.htmlcut.tar");
    let bytes = "\u{feff}<p>é\r\n✓\0</p>".as_bytes();
    std::fs::write(&source, bytes).unwrap();
    std::fs::write(
        &plan,
        htmlcut_core::canonical_json(&htmlcut_core::ExtractionPlan::css("p").unwrap()).unwrap(),
    )
    .unwrap();
    let result = invoke(
        &[
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--plan",
            plan.to_str().unwrap(),
            "--bundle",
            bundle.to_str().unwrap(),
        ],
        b"",
    );
    assert!(result.status.success());
    let entries = members(&bundle);
    assert_eq!(
        entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
        ["manifest.json", "plan.json", "source.html"]
    );
    assert_eq!(entries[2].1, bytes);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_file(plan).unwrap();
    let moved = root.path().join("moved.htmlcut.tar");
    std::fs::rename(bundle, &moved).unwrap();
    let replay = invoke(&["run", moved.to_str().unwrap()], b"");
    assert!(replay.status.success());
    assert_eq!(replay.stdout, result.stdout);
    let collision = invoke(
        &[
            "run",
            moved.to_str().unwrap(),
            "--output",
            moved.to_str().unwrap(),
            "--overwrite",
        ],
        b"",
    );
    assert_eq!(collision.status.code(), Some(2));
    assert!(collision.stdout.is_empty());
}

#[test]
fn repeat_bundle_creation_is_deterministic_and_data_publication_is_separate() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let first = generated(root.path());
    let second = root.path().join("second.htmlcut.tar");
    assert!(
        invoke(
            &[
                "extract",
                "--stdin",
                "--css",
                "p",
                "--bundle",
                second.to_str().unwrap()
            ],
            b"<p>180</p>"
        )
        .status
        .success()
    );
    assert_eq!(
        std::fs::read(first).unwrap(),
        std::fs::read(&second).unwrap()
    );
    let target = root.path().join("data.json");
    let published = invoke(
        &[
            "run",
            second.to_str().unwrap(),
            "--output",
            target.to_str().unwrap(),
        ],
        b"",
    );
    assert!(published.status.success());
    assert!(published.stdout.is_empty());
    assert_eq!(std::fs::read(&target).unwrap(), b"[\"180\"]\n");
    assert_eq!(
        invoke(
            &[
                "run",
                second.to_str().unwrap(),
                "--output",
                target.to_str().unwrap()
            ],
            b""
        )
        .status
        .code(),
        Some(5)
    );
}

#[test]
fn corrupted_execution_configuration_or_evidence_is_not_trusted() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let valid = generated(root.path());
    let base = members(&valid);
    let target = root.path().join("changed.htmlcut.tar");
    for field in [
        "data_sha256",
        "extraction_sha256",
        "source_sha256",
        "plan_sha256",
        "candidate_count",
        "selected_count",
        "data_kind",
    ] {
        let mut entries = base.clone();
        let mut manifest: serde_json::Value = serde_json::from_slice(&entries[0].1).unwrap();
        manifest["receipt"][field] = match field {
            "candidate_count" | "selected_count" => serde_json::json!(2),
            "data_kind" => serde_json::json!("records"),
            _ => serde_json::json!("0".repeat(64)),
        };
        entries[0].1 = serde_json::to_vec(&manifest).unwrap();
        store(&target, &entries);
        fails(&target, 3);
    }
    for mutate in ["metadata", "preparation", "source", "plan"] {
        let mut entries = base.clone();
        if mutate == "source" {
            entries[2].1 = b"<p>other</p>".to_vec();
        } else if mutate == "plan" {
            let mut plan: serde_json::Value = serde_json::from_slice(&entries[1].1).unwrap();
            plan["strategy"]["selector"] = serde_json::json!("aside");
            entries[1].1 = serde_json::to_vec(&plan).unwrap();
        } else {
            let mut manifest: serde_json::Value = serde_json::from_slice(&entries[0].1).unwrap();
            if mutate == "metadata" {
                manifest["metadata"]["base_url"] = serde_json::json!("https://example.test/");
            } else {
                manifest["preparation"]["max_depth"] = serde_json::json!(1024);
            }
            entries[0].1 = serde_json::to_vec(&manifest).unwrap();
        }
        store(&target, &entries);
        fails(&target, 3);
    }
}

#[test]
fn archive_unknown_members_paths_links_and_extensions_are_rejected_without_unpacking() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let valid = generated(root.path());
    let base = members(&valid);
    let target = root.path().join("bad.htmlcut.tar");
    for name in [
        "../manifest.json",
        "/manifest.json",
        "./manifest.json",
        "other.json",
    ] {
        let mut entries = base.clone();
        entries[0].0 = name.into();
        store(&target, &entries);
        fails(&target, 2);
    }
    for kind in [
        tar::EntryType::Symlink,
        tar::EntryType::Link,
        tar::EntryType::Directory,
        tar::EntryType::GNULongName,
        tar::EntryType::XHeader,
    ] {
        let mut entries = base.clone();
        entries[0].2 = kind;
        store(&target, &entries);
        fails(&target, 2);
    }
    let mut entries = base.clone();
    entries.swap(0, 1);
    store(&target, &entries);
    fails(&target, 2);
    let mut entries = base.clone();
    entries[1] = entries[0].clone();
    store(&target, &entries);
    fails(&target, 2);
    let mut entries = base;
    entries.push(("extra".into(), b"secret".to_vec(), tar::EntryType::Regular));
    store(&target, &entries);
    fails(&target, 2);
    assert!(!root.path().join("manifest.json").exists());
    assert!(!root.path().join("outside").exists());
}

#[test]
fn bad_headers_padding_and_hidden_archive_tail_are_rejected() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let valid = generated(root.path());
    let bytes = std::fs::read(valid).unwrap();
    let target = root.path().join("bad.htmlcut.tar");
    let mut damaged = bytes.clone();
    damaged[0] ^= 1;
    std::fs::write(&target, damaged).unwrap();
    fails(&target, 2);
    let mut damaged = bytes.clone();
    let header = tar::Header::from_byte_slice(&damaged[..512]);
    let length = header.size().unwrap() as usize;
    damaged[512 + length] = 1;
    std::fs::write(&target, damaged).unwrap();
    fails(&target, 2);
    let mut damaged = bytes.clone();
    damaged.push(b'X');
    std::fs::write(&target, damaged).unwrap();
    fails(&target, 2);
    std::fs::write(&target, &bytes[..bytes.len() - 512]).unwrap();
    fails(&target, 2);
    std::fs::write(
        &target,
        br#"{"schema":"htmlcut.run","version":2,"source":{"kind":"stdin"}}"#,
    )
    .unwrap();
    fails(&target, 2);
}

#[test]
fn closed_manifest_versions_configuration_and_member_bounds_are_enforced() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let original = generated(root.path());
    let base = members(&original);
    let target = root.path().join("unsupported.htmlcut.tar");
    for (pointer, value, class) in [
        ("/schema", serde_json::json!("other.bundle"), 2),
        ("/version", serde_json::json!(2), 2),
        (
            "/receipt/schema",
            serde_json::json!("htmlcut.extraction.result"),
            2,
        ),
        ("/receipt/version", serde_json::json!(2), 2),
        ("/receipt/semantics", serde_json::json!(2), 2),
        ("/preparation/max_source_bytes", serde_json::json!(0), 2),
        ("/preparation/max_source_bytes", serde_json::json!(1), 4),
    ] {
        let mut entries = base.clone();
        let mut manifest: serde_json::Value = serde_json::from_slice(&entries[0].1).unwrap();
        *manifest.pointer_mut(pointer).unwrap() = value;
        entries[0].1 = serde_json::to_vec(&manifest).unwrap();
        store(&target, &entries);
        fails(&target, class);
    }
    for (member, bytes, class) in [
        (0, b"{}".to_vec(), 2),
        (0, b"{".to_vec(), 2),
        (1, b"{}".to_vec(), 2),
        (2, vec![0xff], 2),
        (0, vec![b' '; 5 * 1024 * 1024 + 1], 4),
        (1, vec![b' '; htmlcut_core::MAX_PLAN_BYTES + 1], 4),
    ] {
        let mut entries = base.clone();
        entries[member].1 = bytes;
        store(&target, &entries);
        fails(&target, class);
    }
    store(&target, &base[..2]);
    fails(&target, 2);
    let file = std::fs::File::create(&target).unwrap();
    file.set_len(56 * 1024 * 1024 + 1).unwrap();
    fails(&target, 4);
}

#[test]
fn header_format_name_padding_and_claimed_extent_cannot_hide_bytes() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let original = generated(root.path());
    let bytes = std::fs::read(original).unwrap();
    let target = root.path().join("header.htmlcut.tar");
    for change in 0..5 {
        let mut header = tar::Header::new_ustar();
        header.as_mut_bytes().copy_from_slice(&bytes[..512]);
        match change {
            0 => header.as_ustar_mut().unwrap().version = *b"01",
            1 => header.as_ustar_mut().unwrap().prefix[0] = b'x',
            2 => header.as_ustar_mut().unwrap().name[99] = b'x',
            3 => header.set_size(5 * 1024 * 1024),
            4 => header.as_mut_bytes()[257] = b'x',
            _ => unreachable!(),
        }
        header.set_cksum();
        let mut damaged = bytes.clone();
        damaged[..512].copy_from_slice(header.as_bytes());
        std::fs::write(&target, damaged).unwrap();
        fails(&target, 2);
    }
    // A non-block-aligned but otherwise all-zero footer is still malformed.
    let mut damaged = bytes;
    damaged.push(0);
    std::fs::write(&target, damaged).unwrap();
    fails(&target, 2);
}

#[test]
fn record_bundle_raw_rejection_and_receipt_republication_preserve_data_shape() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let plan = root.path().join("record.json");
    let bundle = root.path().join("record.htmlcut.tar");
    let receipt = root.path().join("receipt.json");
    std::fs::write(&plan, br#"{"schema":"htmlcut.extraction.plan","version":4,"strategy":{"kind":"css","selector":"article"},"projection":{"kind":"records","fields":[{"name":"text","selector":"p"}]}}"#).unwrap();
    let extracted = invoke(
        &[
            "extract",
            "--stdin",
            "--plan",
            plan.to_str().unwrap(),
            "--bundle",
            bundle.to_str().unwrap(),
        ],
        b"<article><p>180</p></article>",
    );
    assert!(
        extracted.status.success(),
        "{}",
        String::from_utf8_lossy(&extracted.stderr)
    );
    assert_eq!(extracted.stdout, b"[{\"text\":\"180\"}]\n");
    let raw = invoke(&["run", bundle.to_str().unwrap(), "--raw"], b"");
    assert_eq!(raw.status.code(), Some(2));
    assert!(raw.stdout.is_empty());
    let replayed = invoke(
        &[
            "run",
            bundle.to_str().unwrap(),
            "--receipt",
            receipt.to_str().unwrap(),
        ],
        b"",
    );
    assert!(replayed.status.success());
    assert_eq!(replayed.stdout, extracted.stdout);
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
    assert_eq!(receipt["data_kind"], "records");
    assert_eq!(receipt["fields"][0]["projected_count"], 1);
}

#[test]
fn complete_large_range_receipt_replays_above_the_plan_document_byte_cap() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let plan = root.path().join("slices.json");
    let bundle = root.path().join("ranges.htmlcut.tar");
    let count = 12_000;
    std::fs::write(&plan, br#"{"schema":"htmlcut.extraction.plan","version":4,"strategy":{"kind":"slice","start":{"kind":"literal","value":"["},"end":{"kind":"literal","value":"]"}},"selection":{"kind":"all"},"projection":{"kind":"source"},"limits":{"max_selected":20000,"max_work":10000000}}"#).unwrap();
    let original = invoke(
        &[
            "extract",
            "--stdin",
            "--plan",
            plan.to_str().unwrap(),
            "--bundle",
            bundle.to_str().unwrap(),
        ],
        "[x]".repeat(count).as_bytes(),
    );
    assert!(
        original.status.success(),
        "{}",
        String::from_utf8_lossy(&original.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Vec<String>>(&original.stdout).unwrap(),
        vec!["x"; count]
    );
    let entries = members(&bundle);
    assert!(entries[0].1.len() > htmlcut_core::MAX_PLAN_BYTES);
    let manifest: serde_json::Value = serde_json::from_slice(&entries[0].1).unwrap();
    assert_eq!(
        manifest["receipt"]["ranges"].as_array().unwrap().len(),
        count
    );
    std::fs::remove_file(plan).unwrap();
    let replay = invoke(&["run", bundle.to_str().unwrap()], b"");
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    assert_eq!(replay.stdout, original.stdout);
}

#[test]
fn complete_admitted_member_caps_and_each_last_padding_byte_are_checked() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let valid = generated(root.path());
    let base = members(&valid);
    let target = root.path().join("boundary.htmlcut.tar");
    for (index, cap) in [(0, 5_242_880), (1, 262_144)] {
        for size in [cap - 1, cap] {
            let mut entries = base.clone();
            entries[index].1.resize(size, b' ');
            store(&target, &entries);
            let result = invoke(&["run", target.to_str().unwrap()], b"");
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, b"[\"180\"]\n");
        }
    }
    let bytes = std::fs::read(valid).unwrap();
    let mut archive = tar::Archive::new(bytes.as_slice());
    let positions = archive
        .entries()
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let end = entry.raw_file_position() + entry.header().size().unwrap();
            let next = end.div_ceil(512) * 512;
            assert!(end < next);
            (end as usize, next as usize - 1)
        })
        .collect::<Vec<_>>();
    for (start, last) in positions {
        for position in [start, (start + last) / 2, last] {
            let mut damaged = bytes.clone();
            damaged[position] = 1;
            std::fs::write(&target, damaged).unwrap();
            fails(&target, 2);
        }
    }
    // The filesystem-facing admission uses the same inclusive complete-container ceiling.
    for size in [58_720_255, 58_720_256] {
        let file = std::fs::File::create(&target).unwrap();
        file.set_len(size).unwrap();
        fails(&target, 2);
    }
}
