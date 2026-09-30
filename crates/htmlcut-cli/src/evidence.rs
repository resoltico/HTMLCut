//! Borrowed field-selected evidence; large values are never cloned into a smaller audit budget.

use htmlcut_core::{ExtractionPlan, ExtractionResult};
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::command::AuditField;

pub(crate) struct Evidence<'a> {
    pub(crate) fields: &'a [AuditField],
    pub(crate) result: &'a ExtractionResult,
    pub(crate) plan: &'a ExtractionPlan,
}

#[derive(Serialize)]
struct Counts {
    candidates: u32,
    selected: u32,
}

fn name(field: &AuditField) -> &'static str {
    match field {
        AuditField::Plan => "plan",
        AuditField::SourceDigest => "source_sha256",
        AuditField::PlanDigest => "plan_sha256",
        AuditField::ExtractionDigest => "extraction_sha256",
        AuditField::Counts => "counts",
        AuditField::Ranges => "ranges",
        AuditField::Values => "values",
    }
}

impl Serialize for Evidence<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut fields = self.fields.iter().collect::<Vec<_>>();
        fields.sort_by_key(|field| name(field));
        fields.dedup_by_key(|field| name(field));
        let mut map = serializer.serialize_map(Some(fields.len()))?;
        for field in fields {
            match field {
                AuditField::Plan => map.serialize_entry(name(field), self.plan)?,
                AuditField::SourceDigest => {
                    map.serialize_entry(name(field), &self.result.source_sha256)?
                }
                AuditField::PlanDigest => {
                    map.serialize_entry(name(field), &self.result.plan_sha256)?
                }
                AuditField::ExtractionDigest => {
                    map.serialize_entry(name(field), &self.result.extraction_sha256)?
                }
                AuditField::Counts => map.serialize_entry(
                    name(field),
                    &Counts {
                        candidates: self.result.candidate_count,
                        selected: self.result.selected_count,
                    },
                )?,
                AuditField::Ranges => map.serialize_entry(name(field), &self.result.ranges)?,
                AuditField::Values => map.serialize_entry(name(field), &self.result.values)?,
            }
        }
        map.end()
    }
}
