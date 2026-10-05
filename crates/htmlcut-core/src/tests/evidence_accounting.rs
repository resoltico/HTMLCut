// SPDX-License-Identifier: MPL-2.0
//! Evidence views and logical work checked against independently materialized hash inputs.
use super::*;
use sha2::{Digest, Sha256};

fn frame(components: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for component in components {
        bytes.extend_from_slice(&(component.len() as u64).to_be_bytes());
        bytes.extend_from_slice(component);
    }
    bytes
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn captured_views_and_evidence_charge_actual_frames_at_every_byte_block_edge() {
    for padding in 0..=64 {
        let html = format!("<p>A</p><p>B</p><!--{}-->", "x".repeat(padding));
        let base = format!("https://example.test/{}", "a".repeat(padding));
        let metadata = SnapshotMetadata {
            base_url: Some(base.clone()),
        };
        let preparation = PreparationLimits {
            max_elements: 16,
            max_nodes: 32,
            max_depth: 8,
            max_parse_work: 1024,
            max_source_bytes: 1024,
        };
        let document = PreparedDocument::new(
            SourceSnapshot::new(html.clone(), metadata).unwrap(),
            preparation.clone(),
        )
        .unwrap();
        let mut query = ExtractionPlan::css(format!("p/*{}*/", "q".repeat(padding))).unwrap();
        query.match_mode = Match::All;
        query.min = Some(2);
        query.max = Some(2);
        query.limits.max_work = 10_000;
        let compiled = CompiledPlan::compile(&query).unwrap();
        // Alternate cold and warm identities; neither changes logical execution work.
        if padding % 2 == 0 {
            document.snapshot().source_sha256();
            compiled.plan_sha256();
        }
        let result = document.execute(&compiled).unwrap();
        assert_eq!(result.candidate_count(), 2);
        assert_eq!(result.selected_count(), 2);
        assert_eq!(result.snapshot().html(), html);
        assert_eq!(result.preparation_limits(), &preparation);
        assert_eq!(result.normalized_json(), compiled.normalized_json());
        assert_eq!(result.payload(), br#"["A","B"]"#);
        let source_hash = digest(html.as_bytes());
        let query_frame = frame(&[b"htmlcut.plan/6", compiled.normalized_json().as_bytes()]);
        let query_hash = digest(&query_frame);
        let metadata_bytes = serde_json::to_vec(&serde_json::json!({"base_url":base})).unwrap();
        let preparation_bytes = serde_json::to_vec(&serde_json::json!({
            "max_depth":8,"max_elements":16,"max_nodes":32,"max_parse_work":1024,"max_source_bytes":1024
        }))
        .unwrap();
        let semantics = 6_u32.to_be_bytes();
        let execution_frame = frame(&[
            b"htmlcut.extraction/6",
            source_hash.as_bytes(),
            query_hash.as_bytes(),
            &metadata_bytes,
            &preparation_bytes,
            &semantics,
        ]);
        let before = result.budget.remaining();
        let receipt = result.receipt().unwrap();
        assert_eq!(receipt.source_sha256, source_hash);
        assert_eq!(receipt.plan_sha256, query_hash);
        assert_eq!(receipt.extraction_sha256, digest(&execution_frame));
        assert_eq!(receipt.data_sha256, digest(br#"["A","B"]"#));
        let payload = result.receipt_payload().unwrap();
        assert_eq!(
            serde_json::from_slice::<ExecutionReceipt>(payload).unwrap(),
            *receipt
        );
        assert_eq!(payload, serde_json::to_vec(receipt).unwrap());
        assert_ne!(payload.last(), Some(&b'\n'));
        let units = html.as_bytes().chunks(64).len()
            + query_frame.chunks(64).len()
            + execution_frame.chunks(64).len()
            + br#"["A","B"]"#.chunks(64).len()
            + payload.chunks(64).len();
        assert_eq!(
            before - result.budget.remaining(),
            units as u32,
            "padding {padding}"
        );
        let after = result.budget.remaining();
        result.receipt().unwrap();
        result.receipt_payload().unwrap();
        assert_eq!(result.budget.remaining(), after);
    }
}
