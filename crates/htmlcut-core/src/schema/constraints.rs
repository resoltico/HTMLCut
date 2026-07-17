use super::*;

pub(super) fn interop_plan_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<Plan>(INTEROP_PLAN_SCHEMA_REF).and_then(apply_plan_semantic_constraints)
}

pub(super) fn apply_plan_semantic_constraints(
    mut schema: Value,
) -> Result<Value, SchemaExportError> {
    append_semantic_constraints(
        &mut schema,
        INTEROP_PLAN_SCHEMA_REF,
        [plan_dom_canonicalization_constraint()],
    )?;
    Ok(schema)
}

pub(super) fn interop_result_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InteropResult>(INTEROP_RESULT_SCHEMA_REF)
        .and_then(apply_result_semantic_constraints)
}

pub(super) fn apply_result_semantic_constraints(
    mut schema: Value,
) -> Result<Value, SchemaExportError> {
    append_semantic_constraints(
        &mut schema,
        INTEROP_RESULT_SCHEMA_REF,
        [
            result_comparison_text_output_constraint(),
            result_output_value_constraint(&["text"], json!({ "type": "string" })),
            result_output_value_constraint(
                &["structured"],
                json!({
                    "allOf": [
                        { "type": "object" },
                        { "not": { "required": ["comparisonTextOutput"] } }
                    ]
                }),
            ),
            result_output_value_constraint(
                &["inner_html", "outer_html", "selected_html", "attribute"],
                json!({ "type": "string" }),
            ),
        ],
    )?;
    Ok(schema)
}

pub(super) fn interop_error_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InteropError>(INTEROP_ERROR_SCHEMA_REF)
}

/// Adds the H5 relationship that a canonicalization policy denotes a CSS text-comparison plan.
///
/// The ignored-attribute versus measured-attribute equality is intentionally not represented
/// here: standard JSON Schema cannot compare a dynamic array member with a sibling property.
pub(super) fn plan_dom_canonicalization_constraint() -> Value {
    json!({
        "if": {
            "required": ["dom_canonicalization"],
            "properties": {
                "dom_canonicalization": { "$ref": "#/$defs/DomCanonicalization" }
            }
        },
        "then": {
            "required": ["strategy", "output"],
            "properties": {
                "strategy": {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "const": "css_selector" } }
                },
                "output": {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "enum": ["text", "structured"] } }
                }
            }
        }
    })
}

/// Restricts clone-text evidence to CSS text and structured results.
pub(super) fn result_comparison_text_output_constraint() -> Value {
    json!({
        "if": {
            "allOf": [
                {
                    "required": ["strategy_kind"],
                    "properties": { "strategy_kind": { "const": "css_selector" } }
                },
                output_kind_constraint(&["text", "structured"])
            ]
        },
        "else": {
            "properties": {
                "selected_matches": {
                    "items": {
                        "properties": {
                            "comparison_text_output": { "type": "null" }
                        }
                    }
                }
            }
        }
    })
}

/// Constrains the representation of each selected output payload for one or more output kinds.
pub(super) fn result_output_value_constraint(
    output_kinds: &[&str],
    output_value_schema: Value,
) -> Value {
    json!({
        "if": output_kind_constraint(output_kinds),
        "then": {
            "properties": {
                "selected_matches": {
                    "items": {
                        "properties": { "output_value": output_value_schema }
                    }
                }
            }
        }
    })
}

/// Builds a discriminator check for the tagged interop output union.
pub(super) fn output_kind_constraint(output_kinds: &[&str]) -> Value {
    json!({
        "required": ["output"],
        "properties": {
            "output": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "enum": output_kinds } }
            }
        }
    })
}

/// Appends HTMLCut-owned semantic constraints to a derived root schema.
pub(super) fn append_semantic_constraints(
    schema: &mut Value,
    schema_ref: SchemaRef,
    constraints: impl IntoIterator<Item = Value>,
) -> Result<(), SchemaExportError> {
    let Some(schema_object) = schema.as_object_mut() else {
        return Err(SchemaExportError::DerivedSchemaShape {
            schema_name: schema_ref.schema_name,
            schema_version: schema_ref.schema_version,
        });
    };

    match schema_object.get_mut("allOf") {
        Some(Value::Array(existing_constraints)) => {
            existing_constraints.extend(constraints);
        }
        Some(_) => {
            return Err(SchemaExportError::DerivedSchemaShape {
                schema_name: schema_ref.schema_name,
                schema_version: schema_ref.schema_version,
            });
        }
        None => {
            schema_object.insert(
                "allOf".to_owned(),
                Value::Array(constraints.into_iter().collect()),
            );
        }
    }

    Ok(())
}
