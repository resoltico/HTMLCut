// SPDX-License-Identifier: MPL-2.0
//! Convert literal corpus inputs, optionally adding raw admission documents.
use std::{
    fs,
    hash::{DefaultHasher, Hasher},
    path::Path,
};
fn store(directory: &Path, bytes: &[u8]) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    let mut hash = DefaultHasher::new();
    hash.write(bytes);
    let path = directory.join(format!("{:016x}", hash.finish()));
    if path.exists() {
        assert_eq!(fs::read(path)?, bytes, "content hash collision");
    } else {
        fs::write(path, bytes)?;
    }
    Ok(())
}
fn frame(mode: u8, contract: &[u8], page: &[u8]) -> Vec<u8> {
    let length: u16 = contract
        .len()
        .try_into()
        .expect("fuzz contract exceeds u16 framing");
    let mut bytes = vec![mode];
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(contract);
    bytes.extend_from_slice(page);
    bytes
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let destination = args
        .next()
        .expect("usage: fuzz_corpus CORPUS [ADMISSION_DOCUMENTS]");
    let destination = Path::new(&destination);
    let admissions = args.next();
    assert!(args.next().is_none());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let cases = htmlcut_conformance::discover(&root);
    for case in &cases {
        let contract = case.contract.as_deref().unwrap_or("\":scope\"");
        if case.contract.is_some() {
            store(&destination.join("contract_bytes"), contract.as_bytes())?;
        }
        store(
            &destination.join("extract"),
            &frame(
                if case.options.raw { 4 } else { 0 },
                contract.as_bytes(),
                &case.page,
            ),
        )?;
    }
    if let Some(admissions) = admissions {
        let mut paths = fs::read_dir(admissions)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        for path in paths {
            store(
                &destination.join("extract"),
                &frame(129, b"", &fs::read(path)?),
            )?;
        }
    }
    println!(
        "framed {} literal cases; existing admission corpus preserved",
        cases.len()
    );
    Ok(())
}
