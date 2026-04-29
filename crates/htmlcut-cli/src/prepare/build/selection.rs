use std::num::NonZeroUsize;

use htmlcut_core::SelectionSpec;

use crate::args::{CliMatchMode, SelectionArgs};
use crate::error::{CliError, usage_error};
use crate::model::CliErrorCode;

pub(crate) fn resolve_selection_spec(args: &SelectionArgs) -> Result<SelectionSpec, CliError> {
    match args.r#match {
        CliMatchMode::Single => {
            if args.index.is_some() {
                return Err(usage_error(
                    CliErrorCode::MatchIndexConflict,
                    "--index can only be used with --match nth.",
                ));
            }
            Ok(SelectionSpec::single())
        }
        CliMatchMode::First => {
            if args.index.is_some() {
                return Err(usage_error(
                    CliErrorCode::MatchIndexConflict,
                    "--index can only be used with --match nth.",
                ));
            }
            Ok(SelectionSpec::First)
        }
        CliMatchMode::Nth => {
            let Some(index) = args.index else {
                return Err(usage_error(
                    CliErrorCode::MatchIndexRequired,
                    "--index is required with --match nth.",
                ));
            };
            let Some(index) = NonZeroUsize::new(index) else {
                return Err(usage_error(
                    CliErrorCode::MatchIndexInvalid,
                    "--index must be a positive integer.",
                ));
            };
            Ok(SelectionSpec::nth(index))
        }
        CliMatchMode::All => {
            if args.index.is_some() {
                return Err(usage_error(
                    CliErrorCode::MatchIndexConflict,
                    "--index can only be used with --match nth.",
                ));
            }
            Ok(SelectionSpec::All)
        }
    }
}
