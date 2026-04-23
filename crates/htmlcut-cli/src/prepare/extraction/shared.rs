use htmlcut_core::ExtractionStrategy;

use crate::error::CliError;
use crate::lookup;
use crate::prepare::MaterializedDefinition;
use crate::prepare::definition::materialize_extraction_definition;

pub(super) struct OperationCommands {
    pub(super) display: String,
    pub(super) report: String,
}

pub(super) fn materialize_operation_definition<Build>(
    definition_args: &crate::args::DefinitionArgs,
    expected_strategy: ExtractionStrategy,
    operation_id: htmlcut_core::OperationId,
    build_inline: Build,
) -> Result<(OperationCommands, MaterializedDefinition), CliError>
where
    Build: FnOnce() -> Result<
        (
            htmlcut_core::ExtractionRequest,
            htmlcut_core::RuntimeOptions,
        ),
        CliError,
    >,
{
    let commands = OperationCommands {
        display: lookup::operation_display_command(operation_id)?,
        report: lookup::operation_report_command(operation_id)?,
    };
    let materialized = materialize_extraction_definition(
        definition_args,
        expected_strategy,
        &commands.display,
        operation_id,
        build_inline,
    )?;
    Ok((commands, materialized))
}
