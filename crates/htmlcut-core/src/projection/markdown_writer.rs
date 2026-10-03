//! Bounded Markdown syntax and line-prefix delivery, distinct from literal source text.

use super::*;

pub(super) struct MarkdownWriter<'a> {
    output: ValueBuffer<'a>,
    pub(super) prefix: String,
    pending: usize,
    space: bool,
    source_text: bool,
    number_prefix: bool,
    pub(super) headers: usize,
    bold: bool,
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
            headers: 0,
            bold: false,
        }
    }
    pub(super) fn boundary(&mut self, lines: usize) -> Result<(), ExtractionError> {
        self.close_bold()?;
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
        if self.space {
            self.write(" ")?;
            self.space = false;
        }
        if self.headers > 0 && !self.bold {
            self.write("**")?;
            self.bold = true;
        }
        Ok(())
    }
    pub(super) fn close_bold(&mut self) -> Result<(), ExtractionError> {
        if self.bold {
            self.write("**")?;
            self.bold = false;
        }
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
            let escape = "\\`*_[]<>!".contains(c)
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
            if "\\<>".contains(c) {
                self.write("\\")?;
            }
            self.write(c.encode_utf8(&mut [0; 4]))?;
        }
        self.write(">")
    }
    pub(super) fn finish(mut self) -> Result<String, ExtractionError> {
        self.close_bold()?;
        // Pending structural separators and prose whitespace were never emitted.
        Ok(self.output.finish())
    }
}
