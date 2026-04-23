use crate::model::{SchemaDocumentReport, SchemaRefReport};

pub(super) trait SchemaRefLike {
    fn schema_name(&self) -> &str;
    fn schema_version(&self) -> u32;
}

pub(super) fn render_schema_ref(schema: &impl SchemaRefLike) -> String {
    format!("{}@{}", schema.schema_name(), schema.schema_version())
}

impl SchemaRefLike for SchemaRefReport {
    fn schema_name(&self) -> &str {
        &self.schema_name
    }

    fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

impl SchemaRefLike for SchemaDocumentReport {
    fn schema_name(&self) -> &str {
        &self.schema_name
    }

    fn schema_version(&self) -> u32 {
        self.schema_version
    }
}
