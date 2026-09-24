use serde_json::Value;

use htmlcut_core::SchemaStability;

use crate::metadata::DISPLAY_NAME;
use crate::model::SchemaCommandReport;

use super::shared::render_schema_ref;

pub(crate) fn render_schema_text(report: &SchemaCommandReport) -> String {
    let schema_command =
        crate::contract::cli_aux_command_display_command(crate::contract::CliAuxCommandId::Schema);
    let mut lines = vec![
        format!("{DISPLAY_NAME} {}", report.version),
        report.description.clone(),
        format!("Schema profile: {}", report.schema_profile),
    ];

    let schema_count = report.schemas.len();
    lines.push(format!(
        "Registry: {schema_count} schema{}.",
        if schema_count == 1 { "" } else { "s" }
    ));
    lines.push(format!(
        "Use `htmlcut {schema_command} --output index-json` for a lightweight machine-readable inventory or `--output json` for embedded schema documents."
    ));

    if report.schemas.is_empty() {
        return lines.join("\n");
    }

    let single_schema = report.schemas.len() == 1;
    lines.push(if single_schema {
        "Schema:".to_owned()
    } else {
        "Schemas:".to_owned()
    });

    for schema in &report.schemas {
        let surface = match schema.profile.as_deref() {
            Some(profile) => format!("{} {profile}", schema.surface),
            None => schema.surface.clone(),
        };
        lines.push(format!(
            "- {} | {} | {} | {}",
            render_schema_ref(schema),
            surface,
            schema.artifact,
            render_schema_stability(schema.stability)
        ));
        if single_schema {
            lines.push(format!(
                "  json schema keys: {}",
                render_json_schema_keys(&schema.json_schema)
            ));
        }
    }

    lines.join("\n")
}

fn render_schema_stability(stability: SchemaStability) -> &'static str {
    match stability {
        SchemaStability::Versioned => "versioned",
    }
}

fn render_json_schema_keys(value: &Value) -> String {
    value
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>().join(", "))
        .unwrap_or_else(|| "(not-an-object)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_text_handles_empty_singular_profileless_and_nonobject_documents() {
        let mut empty = crate::prepare::build_schema_report(None, None).expect("schema report");
        empty.schemas.clear();
        assert!(render_schema_text(&empty).contains("Registry: 0 schemas."));

        let mut singular = crate::prepare::build_schema_report(None, None).expect("schema report");
        singular.schemas.truncate(1);
        singular.schemas[0].profile = None;
        singular.schemas[0].json_schema = Value::Null;
        let expected_schema_ref = format!(
            "{}@{}",
            singular.schemas[0].schema_name, singular.schemas[0].schema_version
        );
        let rendered = render_schema_text(&singular);
        assert!(rendered.contains("Registry: 1 schema."));
        assert!(rendered.contains("Schema:"));
        assert!(
            rendered.contains(&expected_schema_ref),
            "schema text must preserve the emitted document identity"
        );
        assert!(rendered.contains("| versioned"));
        assert!(rendered.contains("json schema keys: (not-an-object)"));
    }
}
