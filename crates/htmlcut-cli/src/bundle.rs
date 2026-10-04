// SPDX-License-Identifier: MPL-2.0
//! A closed, uncompressed USTAR replay container; members are read, never unpacked.

use std::{
    borrow::Cow,
    cell::RefCell,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::bundle_io::{ObservedReader, archive_failure};
use htmlcut_core::{
    CompiledPlan, ErrorCode, ExecutionReceipt, ExtractionError, ExtractionPlan, PreparationLimits,
    PreparedDocument, SCHEMA_VERSION, SEMANTICS_VERSION, SnapshotMetadata, SourceSnapshot,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_MANIFEST_BYTES: usize = htmlcut_core::MAX_RECEIPT_BYTES + 1024 * 1024;
const MAX_TAIL_BYTES: usize = 10_240;
const MIB: usize = 1024 * 1024;
// Fixed complete-container policy; member/padding admission reserves its mandatory footer.
pub(crate) const MAX_BUNDLE_BYTES: usize = 56 * MIB;
const MEMBERS: [&str; 3] = ["manifest.json", "plan.json", "source.html"];

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest<'a> {
    #[schemars(extend("const" = "htmlcut.bundle"))]
    pub(crate) schema: String,
    #[schemars(extend("const" = 1))]
    pub(crate) version: u32,
    pub(crate) metadata: Cow<'a, SnapshotMetadata>,
    pub(crate) preparation: Cow<'a, PreparationLimits>,
    pub(crate) receipt: Cow<'a, ExecutionReceipt>,
}

pub(crate) struct Replay {
    pub(crate) document: PreparedDocument,
    pub(crate) plan: CompiledPlan,
    pub(crate) expected: ExecutionReceipt,
}

pub(crate) fn invalid() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::InvalidBundle,
        "bundle",
        "The closed replay bundle is malformed or unsupported.",
    )
}

pub(crate) fn write(
    output: &mut impl Write,
    document: &PreparedDocument,
    plan: &CompiledPlan,
    receipt: &ExecutionReceipt,
) -> Result<(), ExtractionError> {
    let manifest = Manifest {
        schema: "htmlcut.bundle".into(),
        version: 1,
        metadata: Cow::Borrowed(document.snapshot().metadata()),
        preparation: Cow::Borrowed(document.preparation_limits()),
        receipt: Cow::Borrowed(receipt),
    };
    let manifest = crate::publication::json_payload(&manifest, MAX_MANIFEST_BYTES)?;
    let plan = crate::publication::json_payload(plan.plan(), htmlcut_core::MAX_PLAN_BYTES)?;
    let source = document.snapshot().html().as_bytes();
    let mut archive = tar::Builder::new(output);
    for (name, bytes) in MEMBERS
        .iter()
        .zip([manifest.as_slice(), plan.as_slice(), source])
    {
        let mut header = tar::Header::new_ustar();
        header.set_mode(0o600);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_size(bytes.len() as u64);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        archive
            .append_data(&mut header, name, bytes)
            .map_err(|error| {
                crate::publication::io_failure(error, htmlcut_core::IoOperation::Publication)
            })?;
    }
    archive.finish().map_err(|error| {
        crate::publication::io_failure(error, htmlcut_core::IoOperation::Publication)
    })
}

pub(crate) fn read(path: &Path) -> Result<Replay, ExtractionError> {
    let file = crate::input::open_file(path)?;
    let size = file.metadata().map_err(crate::input::io_failure)?.len();
    if size > MAX_BUNDLE_BYTES as u64 {
        return Err(crate::input::limit("bundle"));
    }
    read_from(file, size)
}

pub(crate) fn read_from<R: Read + Seek>(reader: R, size: u64) -> Result<Replay, ExtractionError> {
    if size > MAX_BUNDLE_BYTES as u64 {
        return Err(crate::input::limit("bundle"));
    }
    let failures = RefCell::new(None);
    let observed = ObservedReader {
        inner: reader,
        failures: &failures,
        position: 0,
        size,
    };
    let mut archive = tar::Archive::new(observed);
    // A newly constructed archive has position zero, the sole entries() precondition.
    let mut entries = archive
        .entries()
        .expect("new archive starts at zero")
        .raw(true);
    let mut padding = Vec::new();
    let mut offset = 0;
    let manifest_bytes = member(
        &mut entries,
        0,
        MAX_MANIFEST_BYTES,
        size,
        &mut offset,
        &mut padding,
        &failures,
    )?;
    let value = htmlcut_core::parse_closed_json(&manifest_bytes, MAX_MANIFEST_BYTES)?;
    let manifest: Manifest<'static> = serde_json::from_value(value).map_err(|_| invalid())?;
    if manifest.schema != "htmlcut.bundle"
        || manifest.version != 1
        || manifest.receipt.schema != "htmlcut.extraction.receipt"
        || manifest.receipt.version != SCHEMA_VERSION
        || manifest.receipt.semantics != SEMANTICS_VERSION
    {
        return Err(invalid());
    }
    manifest.preparation.validate()?;
    let plan_bytes = member(
        &mut entries,
        1,
        htmlcut_core::MAX_PLAN_BYTES,
        size,
        &mut offset,
        &mut padding,
        &failures,
    )?;
    let plan = CompiledPlan::compile(&ExtractionPlan::from_json(&plan_bytes)?)?;
    if plan.plan_sha256() != manifest.receipt.plan_sha256 {
        return Err(mismatch());
    }
    let source = member(
        &mut entries,
        2,
        manifest.preparation.max_source_bytes as usize,
        size,
        &mut offset,
        &mut padding,
        &failures,
    )?;
    let mut reader = archive.into_inner();
    for (position, bytes) in padding {
        reader
            .seek(SeekFrom::Start(position))
            .map_err(crate::input::io_failure)?;
        let mut buffer = [0; 511];
        reader
            .read_exact(&mut buffer[..bytes])
            .map_err(crate::input::io_failure)?;
        if buffer[..bytes].iter().any(|b| *b != 0) {
            return Err(invalid());
        }
    }
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(crate::input::io_failure)?;
    let tail = crate::input::read_bounded(&mut reader, MAX_TAIL_BYTES)?;
    // Admission reserves two footer blocks; ObservedReader rejects EOF before the captured
    // extent, so a successfully read tail already contains those mandatory bytes.
    if !tail.len().is_multiple_of(512) || tail.iter().any(|b| *b != 0) {
        return Err(invalid());
    }
    let source = String::from_utf8(source).map_err(|_| invalid())?;
    let snapshot = SourceSnapshot::new(source, manifest.metadata.into_owned())?;
    if snapshot.source_sha256() != manifest.receipt.source_sha256 {
        return Err(mismatch());
    }
    let document = PreparedDocument::new(snapshot, manifest.preparation.into_owned())?;
    Ok(Replay {
        document,
        plan,
        expected: manifest.receipt.into_owned(),
    })
}

fn member<R: Read>(
    entries: &mut tar::Entries<'_, R>,
    index: usize,
    maximum: usize,
    size: u64,
    offset: &mut u64,
    padding: &mut Vec<(u64, usize)>,
    failures: &RefCell<Option<ExtractionError>>,
) -> Result<Vec<u8>, ExtractionError> {
    let mut entry = entries
        .next()
        .ok_or_else(invalid)?
        .map_err(|_| archive_failure(failures))?;
    let header = entry.header();
    let format = header.as_ustar().ok_or_else(invalid)?;
    let name = MEMBERS[index].as_bytes();
    // as_ustar already requires the exact USTAR magic and version 00.
    if format.name[..name.len()] != *name || format.name[name.len()..].iter().any(|b| *b != 0) {
        return Err(invalid());
    }
    // The exact zero-padded name and empty USTAR prefix define the entire path;
    // no lossy path decoding or normalization is needed.
    if format.prefix.iter().any(|b| *b != 0) || header.entry_type() != tar::EntryType::Regular {
        return Err(invalid());
    }
    let bytes = header.size().map_err(|_| invalid())?;
    if bytes > maximum as u64 {
        return Err(crate::input::limit("bundle"));
    }
    // Each size is bounded above before this arithmetic; the three-entry inventory keeps
    // positions below 56 MiB. Reserve the complete minimum footer before allocating a member.
    let end = entry.raw_file_position() + bytes;
    let next = end.div_ceil(512) * 512;
    if next + 1024 > size {
        return Err(invalid());
    }
    let mut value = vec![0; bytes as usize];
    entry
        .read_exact(&mut value)
        .map_err(crate::input::io_failure)?;
    padding.push((end, (next - end) as usize));
    *offset = next;
    Ok(value)
}

pub(crate) fn mismatch() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::ReplayMismatch,
        "replay",
        "Recomputed execution differs from the recorded bundle.",
    )
}

#[cfg(test)]
#[path = "tests/bundle_io.rs"]
mod io_tests;
