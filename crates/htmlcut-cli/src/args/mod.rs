use std::ffi::OsStr;
use std::marker::PhantomData;

use clap::builder::{PossibleValue, TypedValueParser};
use clap::{Arg, Command, Parser, Subcommand, error::ErrorKind};
use htmlcut_core::CliChoice;

use crate::help::{
    ROOT_HELP_TEMPLATE, catalog_about, catalog_after_help, inspect_about, root_after_help,
    root_before_help, schema_about, schema_after_help, select_about, select_after_help,
    slice_about, slice_after_help,
};
use crate::metadata::TOOL_NAME;

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
    DefinitionArgs, ExtractOutputArgs, FileWriteArgs, GlobalArgs, InspectOutputArgs, SelectionArgs,
    SliceExtractOutputArgs, SourceArgs,
};

pub(crate) type CliPatternMode = htmlcut_core::PatternMode;
pub(crate) type CliMatchMode = crate::contract::CliSelectionMode;
pub(crate) type CliOutputMode = crate::contract::CliOutputMode;
pub(crate) type CliInspectOutputMode = crate::contract::CliTextJsonOutputMode;
pub(crate) type CliCatalogOutputMode = crate::contract::CliTextJsonOutputMode;
pub(crate) type CliSchemaOutputMode = crate::contract::CliTextJsonOutputMode;
pub(crate) type CliTlsTrustMode = crate::contract::CliTlsTrustMode;
pub(crate) type CliBoundaryRetentionMode = crate::contract::CliBoundaryRetentionMode;
pub(crate) type CliWhitespaceMode = htmlcut_core::WhitespaceMode;
pub(crate) type CliFetchPreflightMode = htmlcut_core::FetchPreflightMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CliValueMode {
    Text,
    InnerHtml,
    OuterHtml,
    Attribute,
    Structured,
}

impl CliChoice for CliValueMode {
    fn variants() -> &'static [Self] {
        const VARIANTS: &[CliValueMode] = &[
            CliValueMode::Text,
            CliValueMode::InnerHtml,
            CliValueMode::OuterHtml,
            CliValueMode::Attribute,
            CliValueMode::Structured,
        ];
        VARIANTS
    }

    fn as_cli_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::InnerHtml => "inner-html",
            Self::OuterHtml => "outer-html",
            Self::Attribute => "attribute",
            Self::Structured => "structured",
        }
    }
}

impl std::fmt::Display for CliValueMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_cli_str())
    }
}

impl From<CliValueMode> for htmlcut_core::ValueType {
    fn from(value: CliValueMode) -> Self {
        match value {
            CliValueMode::Text => Self::Text,
            CliValueMode::InnerHtml => Self::InnerHtml,
            CliValueMode::OuterHtml => Self::OuterHtml,
            CliValueMode::Attribute => Self::Attribute,
            CliValueMode::Structured => Self::Structured,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CliSliceValueMode {
    Text,
    SelectedHtml,
    InnerHtml,
    OuterHtml,
    Attribute,
    Structured,
}

impl CliChoice for CliSliceValueMode {
    fn variants() -> &'static [Self] {
        const VARIANTS: &[CliSliceValueMode] = &[
            CliSliceValueMode::Text,
            CliSliceValueMode::SelectedHtml,
            CliSliceValueMode::InnerHtml,
            CliSliceValueMode::OuterHtml,
            CliSliceValueMode::Attribute,
            CliSliceValueMode::Structured,
        ];
        VARIANTS
    }

    fn as_cli_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::SelectedHtml => "selected-html",
            Self::InnerHtml => "inner-html",
            Self::OuterHtml => "outer-html",
            Self::Attribute => "attribute",
            Self::Structured => "structured",
        }
    }
}

impl std::fmt::Display for CliSliceValueMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_cli_str())
    }
}

impl From<CliSliceValueMode> for htmlcut_core::ValueType {
    fn from(value: CliSliceValueMode) -> Self {
        match value {
            CliSliceValueMode::Text => Self::Text,
            CliSliceValueMode::SelectedHtml => Self::SelectedHtml,
            CliSliceValueMode::InnerHtml => Self::InnerHtml,
            CliSliceValueMode::OuterHtml => Self::OuterHtml,
            CliSliceValueMode::Attribute => Self::Attribute,
            CliSliceValueMode::Structured => Self::Structured,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CliChoiceParser<T: 'static> {
    allowed: &'static [T],
    _marker: PhantomData<T>,
}

pub(crate) fn cli_choice_parser<T>() -> CliChoiceParser<T>
where
    T: CliChoice,
{
    CliChoiceParser {
        allowed: T::variants(),
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
    bin_name = TOOL_NAME,
    before_help = root_before_help(),
    help_template = ROOT_HELP_TEMPLATE,
    after_long_help = root_after_help(),
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
        after_long_help = catalog_after_help()
    )]
    Catalog(CatalogArgs),
    #[command(
        about = schema_about(),
        after_long_help = schema_after_help()
    )]
    Schema(SchemaArgs),
    #[command(
        about = select_about(),
        after_long_help = select_after_help()
    )]
    Select(SelectArgs),
    #[command(
        about = slice_about(),
        after_long_help = slice_after_help()
    )]
    Slice(SliceArgs),
    #[command(about = inspect_about())]
    Inspect(InspectArgs),
}
