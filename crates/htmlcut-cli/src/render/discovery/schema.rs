use serde_json::Value;

use htmlcut_core::SchemaStability;

use crate::model::SchemaCommandReport;

use super::shared::render_schema_ref;

pub(crate) fn render_schema_text(report: &SchemaCommandReport) -> String {
    let schema_command =
        htmlcut_core::cli_aux_command_display_command(htmlcut_core::CliAuxCommandId::Schema);
    let mut lines = vec![
        format!("{} {}", report.tool, report.version),
        report.description.clone(),
        format!("Schema profile: {}", report.schema_profile),
    ];

    let schema_count = report.schemas.len();
    lines.push(format!(
        "Registry: {schema_count} schema{}.",
        if schema_count == 1 { "" } else { "s" }
    ));
    lines.push(format!(
        "Use `htmlcut {schema_command} --name <SCHEMA_NAME> --output json` for one schema family."
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
        lines.push(format!(
            "- {} | {} | {}",
            render_schema_ref(schema),
            schema.owner_surface,
            render_schema_stability(schema.stability)
        ));
        lines.push(format!("  rust: {}", schema.rust_shape));
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
        SchemaStability::Frozen => "frozen",
    }
}

fn render_json_schema_keys(value: &Value) -> String {
    value
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>().join(", "))
        .unwrap_or_else(|| "(not-an-object)".to_owned())
}
