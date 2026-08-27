use super::*;

#[test]
fn cli_value_display_matches_rendered_cli_value() {
    let selection = CliValue::SelectionMode(CliSelectionMode::All);
    let boolean = CliValue::Boolean(true);

    assert_eq!(selection.to_string(), render_cli_value(selection));
    assert_eq!(boolean.to_string(), render_cli_value(boolean));
}

#[test]
fn cli_boundary_retention_modes_map_to_domain_values() {
    assert_eq!(
        BoundaryRetention::from(CliBoundaryRetentionMode::ExcludeBoth),
        BoundaryRetention::ExcludeBoth
    );
    assert_eq!(
        BoundaryRetention::from(CliBoundaryRetentionMode::IncludeStart),
        BoundaryRetention::IncludeStart
    );
    assert_eq!(
        BoundaryRetention::from(CliBoundaryRetentionMode::IncludeEnd),
        BoundaryRetention::IncludeEnd
    );
    assert_eq!(
        BoundaryRetention::from(CliBoundaryRetentionMode::IncludeBoth),
        BoundaryRetention::IncludeBoth
    );
}

#[test]
fn text_json_and_schema_output_modes_map_back_to_general_output_modes() {
    assert_eq!(
        CliTextJsonOutputMode::Text.as_output_mode(),
        CliOutputMode::Text
    );
    assert_eq!(
        CliTextJsonOutputMode::Json.as_output_mode(),
        CliOutputMode::Json
    );
    assert_eq!(
        CliSchemaOutputMode::Text.as_output_mode(),
        CliOutputMode::Text
    );
    assert_eq!(
        CliSchemaOutputMode::Json.as_output_mode(),
        CliOutputMode::Json
    );
    assert_eq!(
        CliSchemaOutputMode::IndexJson.as_output_mode(),
        CliOutputMode::Json
    );
}

#[test]
fn input_descriptions_and_parameter_sections_are_stable_public_copy() {
    assert_eq!(CliInputForm::LocalFilePath.description(), "local file path");
    assert_eq!(CliInputForm::Url.description(), "http:// or https:// URL");
    assert_eq!(CliInputForm::Stdin.description(), "- for stdin");

    for (section, label) in [
        (CliParameterSection::Source, "Source"),
        (CliParameterSection::Definition, "Definition"),
        (CliParameterSection::Selection, "Selection"),
        (CliParameterSection::Extraction, "Extraction"),
        (CliParameterSection::InspectionOutput, "Inspection Output"),
        (CliParameterSection::FilesystemOutput, "Filesystem Output"),
    ] {
        assert_eq!(section.label(), label);
        assert_eq!(section.to_string(), label);
    }
}
