// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn optional_crypto_destination_failure_is_not_silently_skipped() {
    let source = htmlcut_tempdir::tempdir().unwrap();
    let destination = htmlcut_tempdir::tempdir().unwrap();
    for name in VENDORED_SELECTOR_STACK_DIRECTORIES {
        fs::create_dir(source.path().join(name)).unwrap();
        fs::write(source.path().join(name).join("source.rs"), b"fixture").unwrap();
    }
    fs::create_dir(source.path().join("sha2")).unwrap();
    fs::write(source.path().join("sha2/source.rs"), b"crypto fixture").unwrap();
    fs::write(destination.path().join("sha2"), b"blocking file").unwrap();
    assert!(copy_published_vendored_selector_stack(source.path(), destination.path()).is_err());
    assert_eq!(
        fs::read(destination.path().join("sha2")).unwrap(),
        b"blocking file"
    );
}
