use super::OperationCliContract;
use super::operation_specs::{build_cli_operation_contract, operation_surface_specs};

pub(super) fn build_cli_operation_catalog() -> Vec<OperationCliContract> {
    operation_surface_specs()
        .iter()
        .filter_map(build_cli_operation_contract)
        .collect()
}
