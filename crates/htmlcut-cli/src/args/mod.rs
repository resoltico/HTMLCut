use std::ffi::OsStr;
use std::marker::PhantomData;

use clap::builder::{PossibleValue, TypedValueParser};
use clap::{Arg, Command, Parser, Subcommand, error::ErrorKind};
use htmlcut_core::CliChoice;

use crate::help::{
    catalog_about, catalog_after_help, catalog_long_about, inspect_about, inspect_long_about,
    root_after_help, root_long_about, schema_about, schema_after_help, schema_long_about,
    select_about, select_after_help, select_long_about, slice_about, slice_after_help,
    slice_long_about,
};
use crate::metadata::{HTMLCUT_DESCRIPTION, TOOL_NAME};

mod discovery;
mod extract;
mod inspect;
mod shared;

pub(crate) use self::discovery::{CatalogArgs, SchemaArgs};
pub(crate) use self::extract::{SelectArgs, SliceArgs};
pub(crate) use self::inspect::{
    InspectArgs, InspectCommands, InspectSelectArgs, InspectSliceArgs, InspectSourceArgs,
};
pub(crate) use self::shared::{
    DefinitionArgs, ExtractOutputArgs, GlobalArgs, InspectOutputArgs, SelectionArgs, SourceArgs,
};

pub(crate) type CliPatternMode = htmlcut_core::PatternMode;
pub(crate) type CliMatchMode = htmlcut_core::CliSelectionMode;
pub(crate) type CliValueMode = htmlcut_core::ValueType;
pub(crate) type CliOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliInspectOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliCatalogOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliSchemaOutputMode = htmlcut_core::CliOutputMode;
pub(crate) type CliWhitespaceMode = htmlcut_core::WhitespaceMode;
pub(crate) type CliFetchPreflightMode = htmlcut_core::FetchPreflightMode;

pub(crate) const TEXT_JSON_OUTPUT_MODES: &[CliOutputMode] =
    &[CliOutputMode::Text, CliOutputMode::Json];

#[derive(Clone, Copy, Debug)]
pub(crate) struct CliChoiceParser<T: 'static> {
    allowed: &'static [T],
    _marker: PhantomData<T>,
}

pub(crate) fn cli_choice_parser<T>() -> CliChoiceParser<T>
where
    T: CliChoice,
{
    cli_choice_subset_parser(T::variants())
}

pub(crate) fn cli_choice_subset_parser<T>(allowed: &'static [T]) -> CliChoiceParser<T>
where
    T: CliChoice,
{
    CliChoiceParser {
        allowed,
        _marker: PhantomData,
    }
}

impl<T> TypedValueParser for CliChoiceParser<T>
where
    T: CliChoice + Send + Sync,
{
    type Value = T;

    fn parse_ref(
        &self,
        _command: &Command,
        arg: Option<&Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let Some(raw) = value.to_str() else {
            return Err(clap::Error::raw(
                ErrorKind::InvalidUtf8,
                "value is not valid UTF-8",
            ));
        };

        self.allowed
            .iter()
            .copied()
            .find(|candidate| candidate.as_cli_str() == raw)
            .ok_or_else(|| {
                let mut message = match arg {
                    Some(argument) => {
                        format!("invalid value '{raw}' for {}", argument.get_id().as_str())
                    }
                    None => format!("invalid value '{raw}'"),
                };
                let choices = self
                    .allowed
                    .iter()
                    .map(|candidate| candidate.as_cli_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                message.push_str(&format!(" [possible values: {choices}]"));
                clap::Error::raw(ErrorKind::InvalidValue, message)
            })
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            self.allowed
                .iter()
                .map(|candidate| PossibleValue::new(candidate.as_cli_str())),
        ))
    }
}

#[derive(Debug, Parser)]
#[command(
    name = TOOL_NAME,
    about = HTMLCUT_DESCRIPTION,
    long_about = root_long_about(),
    after_help = root_after_help(),
    disable_help_subcommand = true,
    disable_version_flag = true,
    subcommand_required = true
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) global: GlobalArgs,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    #[command(
        about = catalog_about(),
        long_about = catalog_long_about(),
        after_help = catalog_after_help()
    )]
    Catalog(CatalogArgs),
    #[command(
        about = schema_about(),
        long_about = schema_long_about(),
        after_help = schema_after_help()
    )]
    Schema(SchemaArgs),
    #[command(
        about = select_about(),
        long_about = select_long_about(),
        after_help = select_after_help()
    )]
    Select(SelectArgs),
    #[command(
        about = slice_about(),
        long_about = slice_long_about(),
        after_help = slice_after_help()
    )]
    Slice(SliceArgs),
    #[command(about = inspect_about(), long_about = inspect_long_about())]
    Inspect(InspectArgs),
}
