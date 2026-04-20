use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AttributeName, ExtractionStrategy, PatternMode, SelectorQuery, SliceBoundary, ValueType,
};
use crate::contracts::default_regex_flags;

/// Mode-correct pattern contract for slice extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum SlicePatternSpec {
    /// Treat `from` and `to` as literal substrings.
    Literal {
        /// Starting boundary pattern.
        from: SliceBoundary,
        /// Ending boundary pattern.
        to: SliceBoundary,
    },
    /// Treat `from` and `to` as regular expressions.
    Regex {
        /// Starting boundary pattern.
        from: SliceBoundary,
        /// Ending boundary pattern.
        to: SliceBoundary,
        #[serde(default = "default_regex_flags")]
        /// Regex flags applied to both boundaries.
        flags: String,
    },
}

impl SlicePatternSpec {
    /// Builds a literal slice pattern.
    pub const fn literal(from: SliceBoundary, to: SliceBoundary) -> Self {
        Self::Literal { from, to }
    }

    /// Builds a regex slice pattern.
    pub fn regex(from: SliceBoundary, to: SliceBoundary, flags: impl Into<String>) -> Self {
        Self::Regex {
            from,
            to,
            flags: flags.into(),
        }
    }

    /// Returns the slice boundary interpretation mode.
    pub const fn mode(&self) -> PatternMode {
        match self {
            Self::Literal { .. } => PatternMode::Literal,
            Self::Regex { .. } => PatternMode::Regex,
        }
    }

    /// Returns the starting boundary pattern.
    pub const fn from(&self) -> &SliceBoundary {
        match self {
            Self::Literal { from, .. } | Self::Regex { from, .. } => from,
        }
    }

    /// Returns the ending boundary pattern.
    pub const fn to(&self) -> &SliceBoundary {
        match self {
            Self::Literal { to, .. } | Self::Regex { to, .. } => to,
        }
    }

    /// Returns regex flags when regex mode is used.
    pub fn flags(&self) -> Option<&str> {
        match self {
            Self::Literal { .. } => None,
            Self::Regex { flags, .. } => Some(flags.as_str()),
        }
    }
}

/// Match-retention policy for one extraction request.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SelectionSpec {
    /// Require exactly one candidate and fail on ambiguity.
    Single,
    /// Keep the first candidate only.
    #[default]
    First,
    /// Keep one 1-based candidate by index.
    Nth {
        /// 1-based index of the retained candidate.
        index: NonZeroUsize,
    },
    /// Keep every discovered candidate.
    All,
}

impl SelectionSpec {
    /// Builds an exact-one selection spec.
    pub const fn single() -> Self {
        Self::Single
    }

    /// Builds an nth-selection spec.
    pub const fn nth(index: NonZeroUsize) -> Self {
        Self::Nth { index }
    }

    /// Returns the optional retained index for nth-selection mode.
    pub const fn index(&self) -> Option<NonZeroUsize> {
        match self {
            Self::Nth { index } => Some(*index),
            Self::Single | Self::First | Self::All => None,
        }
    }
}

/// Value shape requested for each surviving match.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ValueSpec {
    /// Return normalized or preserved plain text.
    #[default]
    Text,
    /// Return inner HTML.
    InnerHtml,
    /// Return outer HTML.
    OuterHtml,
    /// Return one named attribute value.
    Attribute {
        /// Attribute name to extract from each retained match.
        name: AttributeName,
    },
    /// Return a structured JSON payload with metadata.
    Structured,
}

impl ValueSpec {
    /// Returns the stable value kind for this extraction mode.
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::Text => ValueType::Text,
            Self::InnerHtml => ValueType::InnerHtml,
            Self::OuterHtml => ValueType::OuterHtml,
            Self::Attribute { .. } => ValueType::Attribute,
            Self::Structured => ValueType::Structured,
        }
    }

    /// Returns the configured attribute name when attribute extraction is requested.
    pub fn attribute_name(&self) -> Option<&AttributeName> {
        match self {
            Self::Attribute { name } => Some(name),
            Self::Text | Self::InnerHtml | Self::OuterHtml | Self::Structured => None,
        }
    }
}

/// Literal or regex slice configuration for slice extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SliceSpec {
    #[serde(flatten)]
    /// Literal or regex boundary pattern configuration.
    pub pattern: SlicePatternSpec,
    /// Whether to include the matched start boundary in the selected fragment.
    #[serde(default)]
    pub include_start: bool,
    /// Whether to include the matched end boundary in the selected fragment.
    #[serde(default)]
    pub include_end: bool,
}

impl SliceSpec {
    /// Creates a slice definition with literal defaults that exclude both boundaries.
    pub fn new(from: SliceBoundary, to: SliceBoundary) -> Self {
        Self {
            pattern: SlicePatternSpec::literal(from, to),
            include_start: false,
            include_end: false,
        }
    }

    /// Creates a regex slice definition with defaults that exclude both boundaries.
    pub fn regex(from: SliceBoundary, to: SliceBoundary, flags: impl Into<String>) -> Self {
        Self {
            pattern: SlicePatternSpec::regex(from, to, flags),
            include_start: false,
            include_end: false,
        }
    }

    /// Sets whether the selected fragment should include the matched boundaries.
    pub fn with_boundary_inclusion(mut self, include_start: bool, include_end: bool) -> Self {
        self.include_start = include_start;
        self.include_end = include_end;
        self
    }

    /// Returns the slice boundary interpretation mode.
    pub const fn mode(&self) -> PatternMode {
        self.pattern.mode()
    }

    /// Returns the starting boundary pattern.
    pub const fn from(&self) -> &SliceBoundary {
        self.pattern.from()
    }

    /// Returns the ending boundary pattern.
    pub const fn to(&self) -> &SliceBoundary {
        self.pattern.to()
    }

    /// Returns regex flags when regex mode is used.
    pub fn flags(&self) -> Option<&str> {
        self.pattern.flags()
    }
}

/// Core extraction strategy plus value-shaping configuration.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ExtractionSpec {
    /// Select DOM nodes with a CSS selector.
    Selector {
        /// CSS selector used to find candidate nodes.
        selector: SelectorQuery,
        /// Candidate-retention policy.
        #[serde(default)]
        selection: SelectionSpec,
        /// Value shape produced for each retained candidate.
        #[serde(default)]
        value: ValueSpec,
    },
    /// Slice raw source text with literal or regex boundaries.
    Slice {
        /// Slice boundary configuration.
        #[serde(flatten)]
        slice: SliceSpec,
        /// Candidate-retention policy.
        #[serde(default)]
        selection: SelectionSpec,
        /// Value shape produced for each retained candidate.
        #[serde(default)]
        value: ValueSpec,
    },
}

impl ExtractionSpec {
    /// Builds a selector extraction spec with default selection and value behavior.
    pub fn selector(selector: SelectorQuery) -> Self {
        Self::Selector {
            selector,
            selection: SelectionSpec::default(),
            value: ValueSpec::default(),
        }
    }

    /// Builds a slice extraction spec with default selection and value behavior.
    pub fn slice(slice: SliceSpec) -> Self {
        Self::Slice {
            slice,
            selection: SelectionSpec::default(),
            value: ValueSpec::default(),
        }
    }

    /// Replaces the selection policy for this extraction spec.
    pub fn with_selection(self, selection: SelectionSpec) -> Self {
        match self {
            Self::Selector {
                selector, value, ..
            } => Self::Selector {
                selector,
                selection,
                value,
            },
            Self::Slice { slice, value, .. } => Self::Slice {
                slice,
                selection,
                value,
            },
        }
    }

    /// Replaces the extracted value shape for this extraction spec.
    pub fn with_value(self, value: ValueSpec) -> Self {
        match self {
            Self::Selector {
                selector,
                selection,
                ..
            } => Self::Selector {
                selector,
                selection,
                value,
            },
            Self::Slice {
                slice, selection, ..
            } => Self::Slice {
                slice,
                selection,
                value,
            },
        }
    }

    /// Returns the extraction family represented by this spec.
    pub const fn strategy(&self) -> ExtractionStrategy {
        match self {
            Self::Selector { .. } => ExtractionStrategy::Selector,
            Self::Slice { .. } => ExtractionStrategy::Slice,
        }
    }

    /// Returns the selection policy for this extraction spec.
    pub const fn selection(&self) -> &SelectionSpec {
        match self {
            Self::Selector { selection, .. } | Self::Slice { selection, .. } => selection,
        }
    }

    /// Returns the value shape for this extraction spec.
    pub const fn value(&self) -> &ValueSpec {
        match self {
            Self::Selector { value, .. } | Self::Slice { value, .. } => value,
        }
    }

    /// Returns the selector query when selector extraction is requested.
    pub fn selector_query(&self) -> Option<&SelectorQuery> {
        match self {
            Self::Selector { selector, .. } => Some(selector),
            Self::Slice { .. } => None,
        }
    }

    /// Returns the slice definition when slice extraction is requested.
    pub const fn slice_spec(&self) -> Option<&SliceSpec> {
        match self {
            Self::Slice { slice, .. } => Some(slice),
            Self::Selector { .. } => None,
        }
    }
}
