// SPDX-License-Identifier: MPL-2.0
//! Definition-owned command recovery information without echoing supplied values.

use crate::command::Cli;
use clap::{
    Command, CommandFactory,
    error::{ContextKind, ContextValue, ErrorKind},
};
use htmlcut_core::{ConfigurationProblem, ConfigurationRole, ExtractionError, FailureCause};

fn known_option(command: &Command, context: &str) -> Option<String> {
    for arg in command.get_arguments() {
        if let Some(long) = arg.get_long() {
            let flag = format!("--{long}");
            if context == arg.get_id().as_str()
                || context == flag
                || context
                    .strip_prefix(&flag)
                    .is_some_and(|rest| rest.starts_with(' ') || rest.starts_with('='))
            {
                return Some(flag);
            }
        }
    }
    command
        .get_subcommands()
        .find_map(|command| known_option(command, context))
}

pub(crate) fn failure(error: &clap::Error) -> ExtractionError {
    let problem = match error.kind() {
        ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand => {
            ConfigurationProblem::MissingRequired
        }
        ErrorKind::ArgumentConflict => ConfigurationProblem::ConflictingOptions,
        _ => ConfigurationProblem::InvalidValue,
    };
    let option = if error.kind() == ErrorKind::UnknownArgument {
        None
    } else {
        match error.get(ContextKind::InvalidArg) {
            Some(ContextValue::String(context)) => known_option(&Cli::command(), context),
            _ => None,
        }
    };
    let cause = match option {
        Some(option) => FailureCause::Option { option, problem },
        None => FailureCause::Configuration {
            role: ConfigurationRole::Arguments,
            problem,
        },
    };
    crate::input::options(
        "Command options violate the declared vocabulary or constraints; use --help.",
    )
    .with_cause(cause)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[test]
    fn definition_lookup_requires_an_option_boundary() {
        let definitions = Cli::command();
        for context in ["read", "--read", "--read SECRET", "--read=SECRET"] {
            assert_eq!(known_option(&definitions, context), Some("--read".into()));
        }
        assert_eq!(known_option(&definitions, "--readSECRET"), None);
        assert_eq!(known_option(&definitions, "UNDECLARED_SECRET"), None);
    }

    #[test]
    fn command_constraint_causes_distinguish_missing_and_conflicting_input() {
        for (kind, expected) in [
            (
                ErrorKind::MissingRequiredArgument,
                ConfigurationProblem::MissingRequired,
            ),
            (
                ErrorKind::MissingSubcommand,
                ConfigurationProblem::MissingRequired,
            ),
            (
                ErrorKind::ArgumentConflict,
                ConfigurationProblem::ConflictingOptions,
            ),
            (ErrorKind::InvalidValue, ConfigurationProblem::InvalidValue),
        ] {
            let error = clap::Error::raw(kind, "SYNTHETIC_SECRET");
            let result = failure(&error);
            assert_eq!(
                result.evidence.cause,
                Some(FailureCause::Configuration {
                    role: ConfigurationRole::Arguments,
                    problem: expected,
                })
            );
            assert!(
                !serde_json::to_string(&result)
                    .unwrap()
                    .contains("SYNTHETIC_SECRET")
            );
        }
    }

    #[test]
    fn recovery_names_are_definitions_not_user_values() {
        let error = Cli::try_parse_from([
            "htmlcut",
            "extract",
            "--stdin",
            "--select",
            "p",
            "--read",
            "SYNTHETIC_SECRET",
        ])
        .err()
        .unwrap();
        let value = serde_json::to_string(&failure(&error)).unwrap();
        assert!(value.contains("--read"));
        assert!(!value.contains("SYNTHETIC_SECRET"));
        let error = Cli::try_parse_from(["htmlcut", "--SYNTHETIC_SECRET"])
            .err()
            .unwrap();
        let value = serde_json::to_string(&failure(&error)).unwrap();
        assert!(!value.contains("SYNTHETIC_SECRET"));
    }
}
