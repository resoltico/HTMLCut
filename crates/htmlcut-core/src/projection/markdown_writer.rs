// SPDX-License-Identifier: MPL-2.0
//! Bounded Markdown syntax and line-prefix delivery, distinct from literal source text.

use super::markdown_inline::Styles;
use super::*;

pub(super) struct MarkdownWriter<'a> {
    output: ValueBuffer<'a>,
    pub(super) prefix: String,
    pending: usize,
    space: bool,
    source_text: bool,
    number_prefix: bool,
    pub(super) styles: Styles,
    active_styles: Styles,
}

impl<'a> MarkdownWriter<'a> {
    pub(super) fn new(maximum: usize, budget: &'a SelectorWorkBudget) -> Self {
        Self {
            output: ValueBuffer::new(maximum, budget),
            prefix: String::new(),
            pending: 0,
            space: false,
            source_text: false,
            number_prefix: true,
            styles: Styles::default(),
            active_styles: Styles::default(),
        }
    }
    pub(super) fn boundary(&mut self, lines: usize) -> Result<(), ExtractionError> {
        self.close_styles()?;
        self.pending = self.pending.max(lines);
        self.space = false;
        self.source_text = false;
        self.number_prefix = true;
        Ok(())
    }
    pub(super) fn empty_item(&self) -> bool {
        !self.source_text && self.pending == 0 && self.output.value.ends_with("- ")
    }
    pub(super) fn block_start(&mut self, lines: usize) -> Result<(), ExtractionError> {
        // An empty bullet followed by a blank line ends its item in CommonMark.
        // Start its first block on the marker line so the cell/item remains its owner.
        if self.empty_item() {
            self.space = false;
            Ok(())
        } else {
            self.boundary(lines)
        }
    }
    fn prepare(&mut self) -> Result<(), ExtractionError> {
        if !self.output.value.is_empty() {
            for _ in 0..self.pending {
                self.write("\n")?;
            }
        }
        self.pending = 0;
        Ok(())
    }
    fn write(&mut self, value: &str) -> Result<(), ExtractionError> {
        for part in value.split_inclusive('\n') {
            if self.output.value.is_empty() || self.output.value.ends_with('\n') {
                self.output.push(&self.prefix)?;
            }
            self.output.push(part)?;
        }
        Ok(())
    }
    pub(super) fn syntax(&mut self, value: &str) -> Result<(), ExtractionError> {
        self.prepare()?;
        self.write(value)
    }
    pub(super) fn begin_inline(&mut self) -> Result<(), ExtractionError> {
        self.prepare()?;
        self.transition_styles(true)
    }
    pub(super) fn settle_styles(&mut self) -> Result<(), ExtractionError> {
        self.transition_styles(false)
    }
    fn transition_styles(&mut self, spacing: bool) -> Result<(), ExtractionError> {
        let common = self
            .active_styles
            .0
            .iter()
            .zip(&self.styles.0)
            .take_while(|(active, desired)| active == desired)
            .count();
        for index in (common..2).rev() {
            if let Some(style) = self.active_styles.0[index] {
                self.write(style.close())?;
            }
        }
        if spacing && self.space {
            self.write(" ")?;
            self.space = false;
        }
        for index in common..2 {
            if let Some(style) = self.styles.0[index] {
                self.write(style.open())?;
            }
        }
        self.active_styles = self.styles;
        Ok(())
    }
    pub(super) fn close_styles(&mut self) -> Result<(), ExtractionError> {
        for style in self.active_styles.0.into_iter().rev().flatten() {
            self.write(style.close())?;
        }
        self.active_styles = Styles::default();
        Ok(())
    }
    pub(super) fn inline_separator(&mut self) -> Result<(), ExtractionError> {
        self.space = !self.output.value.is_empty();
        self.begin_inline()
    }
    pub(super) fn mark_atom(&mut self) {
        self.source_text = true;
        self.number_prefix = false;
    }
    pub(super) fn code_span(&mut self, value: &str) -> Result<(), ExtractionError> {
        if value.is_empty() {
            return Ok(());
        }
        crate::execution::charge(self.output.budget, 2 * value.len().div_ceil(64) + 1)?;
        let mut longest = 0;
        let mut run = 0;
        let mut all_spaces = true;
        for c in value.chars() {
            let c = if c == '\n' { ' ' } else { c };
            all_spaces &= c == ' ';
            if c == '`' {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        let edge_space = |c: Option<char>| matches!(c, Some(' ' | '\n'));
        let pad = value.starts_with('`')
            || value.ends_with('`')
            || edge_space(value.chars().next())
                && edge_space(value.chars().next_back())
                && !all_spaces;
        self.begin_inline()?;
        self.fence(longest + 1)?;
        if pad {
            self.literal(" ")?;
        }
        for c in value.chars() {
            let c = if c == '\n' { ' ' } else { c };
            self.literal(c.encode_utf8(&mut [0; 4]))?;
        }
        if pad {
            self.literal(" ")?;
        }
        self.fence(longest + 1)?;
        self.mark_atom();
        Ok(())
    }
    pub(super) fn text(&mut self, value: &str) -> Result<(), ExtractionError> {
        crate::execution::charge(self.output.budget, value.len().div_ceil(64))?;
        for c in value.chars() {
            if c.is_ascii_whitespace() {
                self.space = self.source_text;
                continue;
            }
            self.begin_inline()?;
            let escape = "\\`*_[]<>!&".contains(c)
                || (!self.source_text && "#+-=>".contains(c))
                || (self.number_prefix && matches!(c, '.' | ')'));
            if escape {
                self.write("\\")?;
            }
            self.number_prefix &= c.is_ascii_digit();
            self.write(c.encode_utf8(&mut [0; 4]))?;
            self.source_text = true;
        }
        Ok(())
    }
    pub(super) fn fence(&mut self, length: usize) -> Result<(), ExtractionError> {
        self.prepare()?;
        for _ in 0..length {
            self.write("`")?;
        }
        Ok(())
    }
    pub(super) fn literal(&mut self, value: &str) -> Result<(), ExtractionError> {
        self.prepare()?;
        self.write(value)
    }
    pub(super) fn destination(&mut self, value: &str) -> Result<(), ExtractionError> {
        crate::execution::charge(self.output.budget, value.len().div_ceil(64) + 1)?;
        if value.chars().any(|c| matches!(c, '\n' | '\r' | '\0')) {
            return Err(ExtractionError::new(
                ErrorCode::InvalidRepresentation,
                "projection",
                "Markdown destinations cannot contain line endings or NUL.",
            ));
        }
        self.syntax("<")?;
        for c in value.chars() {
            if "\\<>&".contains(c) {
                self.write("\\")?;
            }
            self.write(c.encode_utf8(&mut [0; 4]))?;
        }
        self.write(">")
    }
    pub(super) fn finish(mut self) -> Result<String, ExtractionError> {
        self.close_styles()?;
        // Pending structural separators and prose whitespace were never emitted.
        Ok(self.output.finish())
    }
}

#[cfg(test)]
mod code_span_work_tests {
    use super::*;

    #[test]
    fn literal_atom_requires_scan_and_emission_allowances() {
        for (units, accepted) in [(3, false), (4, true)] {
            let budget = SelectorWorkBudget::new(units);
            let mut writer = MarkdownWriter::new(128, &budget);
            let result = writer.code_span("x");
            assert_eq!(result.is_ok(), accepted);
            if accepted {
                assert_eq!(writer.finish().unwrap(), "`x`");
            } else {
                assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
            }
        }
        for (units, accepted) in [(6, false), (7, true)] {
            let budget = SelectorWorkBudget::new(units);
            let mut writer = MarkdownWriter::new(128, &budget);
            let payload = "x".repeat(65);
            let result = writer.code_span(&payload);
            assert_eq!(result.is_ok(), accepted);
            if accepted {
                assert_eq!(writer.finish().unwrap(), format!("`{payload}`"));
            } else {
                assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
            }
        }
    }
}
