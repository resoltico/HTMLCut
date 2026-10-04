// SPDX-License-Identifier: MPL-2.0
use super::*;
use std::io::{self, Cursor};

struct FaultReader {
    body: Cursor<Vec<u8>>,
    fail_at: u64,
    read_fault: bool,
    seek_fault: bool,
}
impl Read for FaultReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if self.read_fault && self.body.position() >= self.fail_at {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "SECRET"));
        }
        self.body.read(bytes)
    }
}
impl Seek for FaultReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.seek_fault && self.body.position() >= self.fail_at {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "SECRET"));
        }
        self.body.seek(position)
    }
}
fn valid_bytes() -> Vec<u8> {
    let document = PreparedDocument::new(
        SourceSnapshot::new("<p>180</p>", Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    let result = document.execute(&plan).unwrap();
    let mut bytes = Vec::new();
    write(&mut bytes, &document, &plan, &result.receipt).unwrap();
    bytes
}
#[test]
fn genuine_reader_faults_keep_io_class_and_redact_underlying_text() {
    for (read_fault, seek_fault, fail_at) in
        [(true, false, 0), (true, false, 512), (false, true, 0)]
    {
        let bytes = valid_bytes();
        let size = bytes.len() as u64;
        let reader = FaultReader {
            body: Cursor::new(bytes),
            fail_at,
            read_fault,
            seek_fault,
        };
        let error = read_from(reader, size).err().unwrap();
        assert_eq!(error.code, ErrorCode::Acquisition);
        assert_eq!(error.code.exit_class(), 5);
        assert!(!serde_json::to_string(&error).unwrap().contains("SECRET"));
    }
}
#[test]
fn malformed_known_bytes_and_unexpected_underlying_eof_have_distinct_classes() {
    let mut bytes = valid_bytes();
    bytes[0] ^= 1;
    let size = bytes.len() as u64;
    assert_eq!(
        read_from(Cursor::new(bytes), size).err().unwrap().code,
        ErrorCode::InvalidBundle
    );
    let bytes = valid_bytes();
    let size = bytes.len() as u64;
    assert_eq!(
        read_from(Cursor::new(bytes[..64].to_vec()), size)
            .err()
            .unwrap()
            .code,
        ErrorCode::Acquisition
    );
    assert_eq!(
        read_from(Cursor::new(Vec::<u8>::new()), MAX_BUNDLE_BYTES as u64 + 1)
            .err()
            .unwrap()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn bundle_writing_reports_header_body_and_footer_failures_as_publication() {
    struct CappedWriter {
        bytes: usize,
        maximum: usize,
    }
    impl Write for CappedWriter {
        fn write(&mut self, value: &[u8]) -> io::Result<usize> {
            if self.bytes == self.maximum {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "SECRET"));
            }
            let count = value.len().min(self.maximum - self.bytes);
            self.bytes += count;
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let document = PreparedDocument::new(
        SourceSnapshot::new("<p>180</p>", Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    let receipt = document.execute(&plan).unwrap().receipt;
    let length = valid_bytes().len();
    for maximum in [0, 512, length - 1024] {
        let mut writer = CappedWriter { bytes: 0, maximum };
        let error = write(&mut writer, &document, &plan, &receipt).unwrap_err();
        assert_eq!(error.code, ErrorCode::Publication);
        assert_eq!(error.code.exit_class(), 5);
        assert!(!serde_json::to_string(&error).unwrap().contains("SECRET"));
    }
}

#[test]
fn faults_in_padding_and_footer_reads_or_seeks_remain_io_failures() {
    let bytes = valid_bytes();
    let mut archive = tar::Archive::new(Cursor::new(&bytes));
    let boundaries: Vec<_> = archive
        .entries()
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            entry.raw_file_position() + entry.header().size().unwrap()
        })
        .collect();
    for fail_at in boundaries {
        for (read_fault, seek_fault) in [(true, false), (false, true)] {
            let reader = FaultReader {
                body: Cursor::new(bytes.clone()),
                fail_at,
                read_fault,
                seek_fault,
            };
            assert_eq!(
                read_from(reader, bytes.len() as u64).err().unwrap().code,
                ErrorCode::Acquisition
            );
        }
    }
}

#[test]
fn observed_empty_reads_and_successful_seeks_do_not_record_failures() {
    let failures = RefCell::new(None);
    let mut reader = ObservedReader {
        inner: Cursor::new(b"abc"),
        failures: &failures,
        position: 0,
        size: 3,
    };
    assert_eq!(reader.read(&mut []).unwrap(), 0);
    assert_eq!(reader.seek(SeekFrom::Start(1)).unwrap(), 1);
    let mut buffer = [0; 2];
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(&buffer, b"bc");
    assert_eq!(reader.read(&mut [0]).unwrap(), 0);
    assert!(failures.borrow().is_none());
}

#[test]
fn a_file_growing_after_size_capture_cannot_extend_the_recorded_bundle_extent() {
    let bytes = valid_bytes();
    assert_eq!(
        read_from(Cursor::new(bytes), 500).err().unwrap().code,
        ErrorCode::InvalidBundle
    );
}

#[test]
fn footer_reposition_failure_is_acquisition_not_a_malformed_archive() {
    struct FooterSeekFault {
        body: Cursor<Vec<u8>>,
        footer: u64,
    }
    impl Read for FooterSeekFault {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.body.read(bytes)
        }
    }
    impl Seek for FooterSeekFault {
        fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
            if target == SeekFrom::Start(self.footer) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "SECRET"));
            }
            self.body.seek(target)
        }
    }
    let bytes = valid_bytes();
    let mut archive = tar::Archive::new(Cursor::new(&bytes));
    let final_entry = archive.entries().unwrap().last().unwrap().unwrap();
    let end = final_entry.raw_file_position() + final_entry.header().size().unwrap();
    let reader = FooterSeekFault {
        footer: end.div_ceil(512) * 512,
        body: Cursor::new(bytes.clone()),
    };
    let error = read_from(reader, bytes.len() as u64).err().unwrap();
    assert_eq!(error.code, ErrorCode::Acquisition);
    assert!(!serde_json::to_string(&error).unwrap().contains("SECRET"));
}

#[test]
fn maximum_members_and_full_padding_footer_fit_the_fixed_container_allowance() {
    assert_eq!(MAX_BUNDLE_BYTES, 58_720_256);
    assert_eq!(MAX_MANIFEST_BYTES, 5_242_880);
    let members = [5_242_880_u64, 262_144, 52_428_800];
    let required: u64 = members
        .iter()
        .map(|bytes| 512 + bytes.div_ceil(512) * 512)
        .sum::<u64>()
        + MAX_TAIL_BYTES as u64;
    assert!(required <= MAX_BUNDLE_BYTES as u64);
    for size in [MAX_BUNDLE_BYTES as u64 - 1, MAX_BUNDLE_BYTES as u64] {
        // A complete captured extent below/at the allowance is malformed, not over budget.
        assert_eq!(
            read_from(Cursor::new(vec![0; 512]), size)
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidBundle
        );
    }
    assert_eq!(
        read_from(Cursor::new(vec![0; 512]), MAX_BUNDLE_BYTES as u64 + 1)
            .err()
            .unwrap()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn a_final_member_with_exact_minimum_footer_is_admitted() {
    let bytes = valid_bytes();
    let mut archive = tar::Archive::new(Cursor::new(&bytes));
    let final_entry = archive.entries().unwrap().last().unwrap().unwrap();
    let end = final_entry.raw_file_position() + final_entry.header().size().unwrap();
    let minimal = end.div_ceil(512) * 512 + 1024;
    let replay = read_from(Cursor::new(&bytes), minimal).unwrap();
    assert_eq!(
        replay.document.execute(&replay.plan).unwrap().receipt,
        replay.expected
    );
    let truncated = bytes[..(minimal - 1) as usize].to_vec();
    assert_eq!(
        read_from(Cursor::new(truncated), minimal - 1)
            .err()
            .unwrap()
            .code,
        ErrorCode::InvalidBundle
    );
}

#[test]
fn bundle_roundtrip_preserves_the_actual_nondefault_preparation_policy() {
    let policy = PreparationLimits {
        max_source_bytes: 1024,
        max_depth: 64,
        ..Default::default()
    };
    let document = PreparedDocument::new(
        SourceSnapshot::new("<p>value</p>", Default::default()).unwrap(),
        policy.clone(),
    )
    .unwrap();
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    let expected = document.execute(&plan).unwrap();
    let mut bytes = Vec::new();
    write(&mut bytes, &document, &plan, &expected.receipt).unwrap();
    let replay = read_from(Cursor::new(&bytes), bytes.len() as u64).unwrap();
    assert_eq!(replay.document.preparation_limits(), &policy);
    assert_eq!(replay.document.execute(&replay.plan).unwrap(), expected);
    assert_eq!(replay.expected, expected.receipt);
}
